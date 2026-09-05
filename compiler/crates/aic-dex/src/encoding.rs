use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DexError {
    ArithmeticOverflow,
    InvalidAlignment(usize),
    InvalidOffset { offset: usize, width: usize },
    InvalidInput(&'static str),
    IndexOverflow(&'static str),
}

impl fmt::Display for DexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => write!(f, "DEX size arithmetic overflow"),
            Self::InvalidAlignment(value) => {
                write!(f, "alignment must be a non-zero power of two, got {value}")
            }
            Self::InvalidOffset { offset, width } => {
                write!(f, "write of {width} bytes at invalid offset {offset}")
            }
            Self::InvalidInput(reason) => write!(f, "invalid DEX input: {reason}"),
            Self::IndexOverflow(kind) => write!(f, "too many {kind} entries for the DEX format"),
        }
    }
}

impl Error for DexError {}

#[derive(Clone, Debug, Default)]
pub struct ByteWriter {
    pub(crate) bytes: Vec<u8>,
}

impl ByteWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    #[must_use]
    pub fn position(&self) -> usize {
        self.bytes.len()
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend(value.to_le_bytes());
    }
    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend(value.to_le_bytes());
    }
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    /// Pads the output with zeroes up to the requested power-of-two alignment.
    ///
    /// # Errors
    ///
    /// Returns [`DexError::InvalidAlignment`] for a non-power-of-two value or
    /// [`DexError::ArithmeticOverflow`] if the resulting position overflows.
    pub fn align(&mut self, alignment: usize) -> Result<(), DexError> {
        if !alignment.is_power_of_two() {
            return Err(DexError::InvalidAlignment(alignment));
        }
        let padding = self.position().wrapping_neg() & (alignment - 1);
        self.bytes.resize(
            self.position()
                .checked_add(padding)
                .ok_or(DexError::ArithmeticOverflow)?,
            0,
        );
        Ok(())
    }

    /// Replaces a previously written little-endian `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`DexError::InvalidOffset`] when four bytes are not available
    /// at `offset`, or [`DexError::ArithmeticOverflow`] for offset overflow.
    pub fn patch_u32(&mut self, offset: usize, value: u32) -> Result<(), DexError> {
        let end = offset.checked_add(4).ok_or(DexError::ArithmeticOverflow)?;
        let target = self
            .bytes
            .get_mut(offset..end)
            .ok_or(DexError::InvalidOffset { offset, width: 4 })?;
        target.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}

#[must_use]
pub fn encode_uleb128(mut value: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity(5);
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        result.push(byte);
        if value == 0 {
            return result;
        }
    }
}

#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn encode_sleb128(mut value: i32) -> Vec<u8> {
    let mut result = Vec::with_capacity(5);
    loop {
        let byte = (value as u8) & 0x7f;
        value >>= 7;
        let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
        result.push(if done { byte } else { byte | 0x80 });
        if done {
            return result;
        }
    }
}

/// Encodes UTF-16 code units as DEX modified UTF-8, including the terminator.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn encode_mutf8(value: &str) -> Vec<u8> {
    let mut result = Vec::new();
    for unit in value.encode_utf16() {
        match unit {
            0 => result.extend([0xc0, 0x80]),
            0x01..=0x7f => result.push(unit as u8),
            0x80..=0x7ff => result.extend([0xc0 | (unit >> 6) as u8, 0x80 | (unit & 0x3f) as u8]),
            _ => result.extend([
                0xe0 | (unit >> 12) as u8,
                0x80 | ((unit >> 6) & 0x3f) as u8,
                0x80 | (unit & 0x3f) as u8,
            ]),
        }
    }
    result.push(0);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb128_examples_and_boundaries() {
        assert_eq!(encode_uleb128(0), [0]);
        assert_eq!(encode_uleb128(0x7f), [0x7f]);
        assert_eq!(encode_uleb128(0x80), [0x80, 1]);
        assert_eq!(encode_uleb128(0x3fff), [0xff, 0x7f]);
        assert_eq!(encode_uleb128(u32::MAX), [0xff, 0xff, 0xff, 0xff, 0x0f]);
        assert_eq!(encode_sleb128(0), [0]);
        assert_eq!(encode_sleb128(-1), [0x7f]);
        assert_eq!(encode_sleb128(63), [0x3f]);
        assert_eq!(encode_sleb128(64), [0xc0, 0]);
        assert_eq!(encode_sleb128(-65), [0xbf, 0x7f]);
    }

    #[test]
    fn mutf8_handles_null_bmp_and_supplementary_values() {
        assert_eq!(encode_mutf8("A\0é"), [0x41, 0xc0, 0x80, 0xc3, 0xa9, 0]);
        assert_eq!(encode_mutf8("😀"), [0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80, 0]);
    }

    #[test]
    fn checked_alignment_and_patch() {
        let mut writer = ByteWriter::new();
        writer.write_u8(1);
        writer.align(4).unwrap();
        assert_eq!(writer.bytes(), [1, 0, 0, 0]);
        assert!(matches!(
            writer.align(3),
            Err(DexError::InvalidAlignment(3))
        ));
        assert!(matches!(
            writer.patch_u32(2, 9),
            Err(DexError::InvalidOffset { .. })
        ));
    }
}
