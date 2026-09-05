use aic_ir::MinimalClass;

use crate::{
    encoding::{encode_mutf8, encode_uleb128, ByteWriter, DexError},
    integrity::{adler32, sha1},
};

const HEADER_SIZE: u32 = 0x70;
const NO_INDEX: u32 = u32::MAX;
const ENDIAN_CONSTANT: u32 = 0x1234_5678;
const MAP_ENTRIES: u32 = 10;

#[derive(Clone, Copy)]
struct Section {
    kind: u16,
    count: u32,
    offset: u32,
}

#[derive(Debug)]
struct Indexes {
    strings: Vec<String>,
    types: Vec<u32>,
    class_type: u16,
    object_type: u16,
    void_type: u16,
    void_string: u32,
    init_name: u32,
    own_method: u32,
    object_method: u32,
}

impl Indexes {
    fn build(class: &MinimalClass) -> Result<Self, DexError> {
        let mut strings = Vec::from([
            "<init>".to_owned(),
            class.descriptor().to_owned(),
            "Ljava/lang/Object;".to_owned(),
            "V".to_owned(),
        ]);
        strings.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        strings.dedup();
        let string_index = |needle: &str| -> Result<u32, DexError> {
            strings
                .binary_search_by(|candidate| candidate.encode_utf16().cmp(needle.encode_utf16()))
                .map_err(|_| DexError::IndexOverflow("string"))
                .and_then(|v| u32::try_from(v).map_err(|_| DexError::IndexOverflow("string")))
        };
        let mut types = [class.descriptor(), "Ljava/lang/Object;", "V"]
            .into_iter()
            .map(string_index)
            .collect::<Result<Vec<_>, _>>()?;
        types.sort_unstable();
        types.dedup();
        let type_index = |descriptor: &str| -> Result<u16, DexError> {
            let descriptor_idx = string_index(descriptor)?;
            types
                .binary_search(&descriptor_idx)
                .map_err(|_| DexError::IndexOverflow("type"))
                .and_then(|v| u16::try_from(v).map_err(|_| DexError::IndexOverflow("type")))
        };
        let class_type = type_index(class.descriptor())?;
        let object_type = type_index("Ljava/lang/Object;")?;
        if class_type == object_type {
            return Err(DexError::InvalidInput(
                "the generated class cannot be java.lang.Object",
            ));
        }
        let (own_method, object_method) = if class_type < object_type {
            (0, 1)
        } else {
            (1, 0)
        };
        Ok(Self {
            class_type,
            object_type,
            void_type: type_index("V")?,
            void_string: string_index("V")?,
            init_name: string_index("<init>")?,
            own_method,
            object_method,
            strings,
            types,
        })
    }
}

#[derive(Debug)]
struct Layout {
    string_ids: u32,
    type_ids: u32,
    proto_ids: u32,
    method_ids: u32,
    class_defs: u32,
    data: u32,
    string_data: u32,
    code: u32,
    class_data: u32,
    map: u32,
    file_size: u32,
    string_offsets: Vec<u32>,
}

fn to_u32(value: usize) -> Result<u32, DexError> {
    u32::try_from(value).map_err(|_| DexError::ArithmeticOverflow)
}

fn layout(indexes: &Indexes) -> Result<Layout, DexError> {
    let mut cursor = HEADER_SIZE;
    let string_ids = cursor;
    cursor += to_u32(indexes.strings.len())?
        .checked_mul(4)
        .ok_or(DexError::ArithmeticOverflow)?;
    let type_ids = cursor;
    cursor += to_u32(indexes.types.len())?
        .checked_mul(4)
        .ok_or(DexError::ArithmeticOverflow)?;
    let proto_ids = cursor;
    cursor += 12;
    let method_ids = cursor;
    cursor += 16;
    let class_defs = cursor;
    cursor += 32;
    let data = cursor;
    let string_data = cursor;
    let mut string_offsets = Vec::with_capacity(indexes.strings.len());
    for value in &indexes.strings {
        string_offsets.push(cursor);
        let utf16_len = u32::try_from(value.encode_utf16().count())
            .map_err(|_| DexError::ArithmeticOverflow)?;
        cursor = cursor
            .checked_add(to_u32(
                encode_uleb128(utf16_len).len() + encode_mutf8(value).len(),
            )?)
            .ok_or(DexError::ArithmeticOverflow)?;
    }
    cursor = (cursor + 3) & !3;
    let code = cursor;
    cursor += 24;
    let class_data = cursor;
    // Four zero counts, then method index diff, access flags, and code offset.
    cursor += to_u32(
        4 + encode_uleb128(indexes.own_method).len()
            + encode_uleb128(0x1_0001).len()
            + encode_uleb128(code).len(),
    )?;
    cursor = (cursor + 3) & !3;
    let map = cursor;
    cursor += 4 + MAP_ENTRIES * 12;
    Ok(Layout {
        string_ids,
        type_ids,
        proto_ids,
        method_ids,
        class_defs,
        data,
        string_data,
        code,
        class_data,
        map,
        file_size: cursor,
        string_offsets,
    })
}

/// Emits a DEX 035 file defining the requested class and its public constructor.
///
/// # Errors
///
/// Returns a typed [`DexError`] when input cannot be represented, layout
/// arithmetic overflows, or a checked write would exceed its target.
#[allow(clippy::too_many_lines)]
pub fn encode_minimal_dex(class: &MinimalClass) -> Result<Vec<u8>, DexError> {
    let indexes = Indexes::build(class)?;
    let layout = layout(&indexes)?;
    let mut out = ByteWriter::new();
    out.write_bytes(b"dex\n035\0");
    out.write_u32(0);
    out.write_bytes(&[0; 20]);
    out.write_u32(layout.file_size);
    out.write_u32(HEADER_SIZE);
    out.write_u32(ENDIAN_CONSTANT);
    out.write_u32(0);
    out.write_u32(0);
    out.write_u32(layout.map);
    write_size_offset(&mut out, indexes.strings.len(), layout.string_ids)?;
    write_size_offset(&mut out, indexes.types.len(), layout.type_ids)?;
    write_size_offset(&mut out, 1, layout.proto_ids)?;
    write_size_offset(&mut out, 0, 0)?;
    write_size_offset(&mut out, 2, layout.method_ids)?;
    write_size_offset(&mut out, 1, layout.class_defs)?;
    out.write_u32(layout.file_size - layout.data);
    out.write_u32(layout.data);

    for &offset in &layout.string_offsets {
        out.write_u32(offset);
    }
    for &descriptor_idx in &indexes.types {
        out.write_u32(descriptor_idx);
    }
    out.write_u32(indexes.void_string);
    out.write_u32(u32::from(indexes.void_type));
    out.write_u32(0);

    // method_ids are ordered by class_idx, then name_idx, then proto_idx.
    let method_classes = if indexes.class_type < indexes.object_type {
        [indexes.class_type, indexes.object_type]
    } else {
        [indexes.object_type, indexes.class_type]
    };
    for class_type in method_classes {
        out.write_u16(class_type);
        out.write_u16(0);
        out.write_u32(indexes.init_name);
    }

    out.write_u32(u32::from(indexes.class_type));
    out.write_u32(0x0001);
    out.write_u32(u32::from(indexes.object_type));
    out.write_u32(0);
    out.write_u32(NO_INDEX);
    out.write_u32(0);
    out.write_u32(layout.class_data);
    out.write_u32(0);

    for (value, &expected) in indexes.strings.iter().zip(&layout.string_offsets) {
        debug_assert_eq!(to_u32(out.position())?, expected);
        out.write_bytes(&encode_uleb128(
            u32::try_from(value.encode_utf16().count())
                .map_err(|_| DexError::ArithmeticOverflow)?,
        ));
        out.write_bytes(&encode_mutf8(value));
    }
    out.align(4)?;
    debug_assert_eq!(to_u32(out.position())?, layout.code);
    out.write_u16(1);
    out.write_u16(1);
    out.write_u16(1);
    out.write_u16(0);
    out.write_u32(0);
    out.write_u32(4);
    out.write_u16(0x1070);
    out.write_u16(
        u16::try_from(indexes.object_method).map_err(|_| DexError::IndexOverflow("method"))?,
    );
    out.write_u16(0);
    out.write_u16(0x000e);

    debug_assert_eq!(to_u32(out.position())?, layout.class_data);
    out.write_bytes(&[0, 0, 1, 0]); // field counts, direct method count, virtual method count
    out.write_bytes(&encode_uleb128(indexes.own_method));
    out.write_bytes(&encode_uleb128(0x1_0001));
    out.write_bytes(&encode_uleb128(layout.code));
    out.align(4)?;
    debug_assert_eq!(to_u32(out.position())?, layout.map);
    let sections = [
        Section {
            kind: 0x0000,
            count: 1,
            offset: 0,
        },
        Section {
            kind: 0x0001,
            count: to_u32(indexes.strings.len())?,
            offset: layout.string_ids,
        },
        Section {
            kind: 0x0002,
            count: to_u32(indexes.types.len())?,
            offset: layout.type_ids,
        },
        Section {
            kind: 0x0003,
            count: 1,
            offset: layout.proto_ids,
        },
        Section {
            kind: 0x0005,
            count: 2,
            offset: layout.method_ids,
        },
        Section {
            kind: 0x0006,
            count: 1,
            offset: layout.class_defs,
        },
        Section {
            kind: 0x2002,
            count: to_u32(indexes.strings.len())?,
            offset: layout.string_data,
        },
        Section {
            kind: 0x2001,
            count: 1,
            offset: layout.code,
        },
        Section {
            kind: 0x2000,
            count: 1,
            offset: layout.class_data,
        },
        Section {
            kind: 0x1000,
            count: 1,
            offset: layout.map,
        },
    ];
    out.write_u32(to_u32(sections.len())?);
    for section in sections {
        out.write_u16(section.kind);
        out.write_u16(0);
        out.write_u32(section.count);
        out.write_u32(section.offset);
    }
    debug_assert_eq!(to_u32(out.position())?, layout.file_size);

    let signature = sha1(&out.bytes()[32..]);
    out.bytes[12..32].copy_from_slice(&signature);
    let checksum = adler32(&out.bytes()[12..]);
    out.patch_u32(8, checksum)?;
    Ok(out.into_bytes())
}

fn write_size_offset(out: &mut ByteWriter, size: usize, offset: u32) -> Result<(), DexError> {
    out.write_u32(to_u32(size)?);
    out.write_u32(offset);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{adler32, sha1};

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn output_is_deterministic_and_integrity_fields_match() {
        let class = MinimalClass::new("Ldev/aic/Minimal;").unwrap();
        let first = encode_minimal_dex(&class).unwrap();
        let second = encode_minimal_dex(&class).unwrap();
        assert_eq!(first, second);
        assert_eq!(&first[..8], b"dex\n035\0");
        assert_eq!(read_u32(&first, 32) as usize, first.len());
        assert_eq!(&first[12..32], &sha1(&first[32..]));
        assert_eq!(read_u32(&first, 8), adler32(&first[12..]));
    }

    #[test]
    fn indexes_are_sorted_and_deduplicated() {
        let indexes = Indexes::build(&MinimalClass::new("Ldev/aic/Minimal;").unwrap()).unwrap();
        assert!(indexes
            .strings
            .windows(2)
            .all(|v| v[0].encode_utf16().cmp(v[1].encode_utf16()).is_lt()));
        assert!(indexes.types.windows(2).all(|v| v[0] < v[1]));
    }

    #[test]
    fn rejects_a_root_class_redefinition_without_panicking() {
        let class = MinimalClass::new("Ljava/lang/Object;").unwrap();
        assert!(matches!(
            encode_minimal_dex(&class),
            Err(DexError::InvalidInput(_))
        ));
    }

    #[test]
    fn aligned_sections_and_map_are_layout_derived() {
        let bytes = encode_minimal_dex(&MinimalClass::new("Ldev/aic/Minimal;").unwrap()).unwrap();
        let map_off = read_u32(&bytes, 52) as usize;
        assert_eq!(map_off % 4, 0);
        let count = read_u32(&bytes, map_off) as usize;
        let offsets: Vec<_> = (0..count)
            .map(|i| read_u32(&bytes, map_off + 4 + i * 12 + 8))
            .collect();
        assert!(offsets.windows(2).all(|v| v[0] < v[1]));
    }
}
