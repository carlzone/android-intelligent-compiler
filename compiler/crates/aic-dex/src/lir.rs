#![allow(
    clippy::cast_sign_loss,
    clippy::missing_panics_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::too_many_lines
)]
use std::collections::BTreeMap;

use crate::DexError;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValueKind {
    I32,
    Bool,
    Reference,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Register {
    pub index: u8,
    pub kind: ValueKind,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Label(pub u16);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Label(Label),
    Const4 {
        dst: Register,
        value: i8,
    },
    Const16 {
        dst: Register,
        value: i16,
    },
    Const32 {
        dst: Register,
        value: i32,
    },
    ConstString {
        dst: Register,
        string: u16,
    },
    Move {
        dst: Register,
        src: Register,
    },
    MoveObject {
        dst: Register,
        src: Register,
    },
    NewInstance {
        dst: Register,
        ty: u16,
    },
    IGet {
        dst: Register,
        object: Register,
        field: u16,
    },
    IPut {
        src: Register,
        object: Register,
        field: u16,
    },
    AddInt {
        dst: Register,
        left: Register,
        right: Register,
    },
    SubInt {
        dst: Register,
        left: Register,
        right: Register,
    },
    MulInt {
        dst: Register,
        left: Register,
        right: Register,
    },
    DivInt {
        dst: Register,
        left: Register,
        right: Register,
    },
    RemInt {
        dst: Register,
        left: Register,
        right: Register,
    },
    RemIntLit8 {
        dst: Register,
        src: Register,
        value: i8,
    },
    AddInt2Addr {
        dst: Register,
        src: Register,
    },
    AddIntLit8 {
        dst: Register,
        src: Register,
        value: i8,
    },
    IfGe {
        left: Register,
        right: Register,
        target: Label,
    },
    IfNez {
        value: Register,
        target: Label,
    },
    IfEqz {
        value: Register,
        target: Label,
    },
    IfLt {
        left: Register,
        right: Register,
        target: Label,
    },
    IfLe {
        left: Register,
        right: Register,
        target: Label,
    },
    IfGt {
        left: Register,
        right: Register,
        target: Label,
    },
    IfNe {
        left: Register,
        right: Register,
        target: Label,
    },
    InvokeStatic {
        method: u16,
        args: Vec<Register>,
    },
    InvokeVirtual {
        method: u16,
        args: Vec<Register>,
    },
    InvokeSuper {
        method: u16,
        args: Vec<Register>,
    },
    InvokeDirect {
        method: u16,
        args: Vec<Register>,
    },
    MoveResult {
        dst: Register,
    },
    MoveResultObject {
        dst: Register,
    },
    Goto16 {
        target: Label,
    },
    Return {
        value: Register,
    },
    ReturnObject {
        value: Register,
    },
    ReturnVoid,
}

fn width(instruction: &Instruction) -> u32 {
    match instruction {
        Instruction::Label(_) => 0,
        Instruction::Const4 { .. }
        | Instruction::Move { .. }
        | Instruction::MoveObject { .. }
        | Instruction::MoveResult { .. }
        | Instruction::MoveResultObject { .. }
        | Instruction::AddInt2Addr { .. }
        | Instruction::Return { .. }
        | Instruction::ReturnObject { .. }
        | Instruction::ReturnVoid => 1,
        Instruction::Const32 { .. }
        | Instruction::InvokeStatic { .. }
        | Instruction::InvokeVirtual { .. }
        | Instruction::InvokeSuper { .. }
        | Instruction::InvokeDirect { .. } => 3,
        _ => 2,
    }
}
fn reference(register: Register) -> Result<u8, DexError> {
    if register.index > 15 {
        return Err(DexError::InvalidInput("M2 register index exceeds 15"));
    }
    if register.kind == ValueKind::Reference {
        Ok(register.index)
    } else {
        Err(DexError::InvalidInput(
            "object instruction received a scalar register",
        ))
    }
}
fn integer(register: Register) -> Result<u8, DexError> {
    if register.index > 15 {
        return Err(DexError::InvalidInput("M2 register index exceeds 15"));
    }
    if matches!(register.kind, ValueKind::I32 | ValueKind::Bool) {
        Ok(register.index)
    } else {
        Err(DexError::InvalidInput(
            "integer instruction received a reference register",
        ))
    }
}
fn branch(labels: &BTreeMap<Label, u32>, target: Label, at: u32) -> Result<u16, DexError> {
    let destination = *labels
        .get(&target)
        .ok_or(DexError::InvalidInput("undefined DEX label"))?;
    let delta = i64::from(destination) - i64::from(at);
    let value = i16::try_from(delta)
        .map_err(|_| DexError::InvalidInput("DEX branch offset exceeds i16"))?;
    Ok(u16::from_ne_bytes(value.to_ne_bytes()))
}

/// Resolves symbolic labels and assembles the supported M2 instruction subset.
///
/// # Errors
/// Returns a typed error for duplicate/missing labels, invalid register kinds or
/// indices, invalid literals, and branch offsets outside the supported range.
pub fn assemble(instructions: &[Instruction]) -> Result<Vec<u16>, DexError> {
    let mut labels = BTreeMap::new();
    let mut cursor = 0_u32;
    for instruction in instructions {
        if let Instruction::Label(label) = instruction {
            if labels.insert(*label, cursor).is_some() {
                return Err(DexError::InvalidInput("duplicate DEX label"));
            }
        } else {
            cursor = cursor
                .checked_add(width(instruction))
                .ok_or(DexError::ArithmeticOverflow)?
        }
    }
    let mut output =
        Vec::with_capacity(usize::try_from(cursor).map_err(|_| DexError::ArithmeticOverflow)?);
    cursor = 0;
    for instruction in instructions {
        match instruction {
            Instruction::Label(_) => continue,
            Instruction::Const4 { dst, value } => {
                let dst = integer(*dst)?;
                if !(-8..=7).contains(value) {
                    return Err(DexError::InvalidInput("const/4 literal outside range"));
                }
                let literal = (*value as u8) & 0x0f;
                output.push(0x12 | u16::from(dst) << 8 | u16::from(literal) << 12)
            }
            Instruction::Const16 { dst, value } => output.extend([
                0x13 | u16::from(integer(*dst)?) << 8,
                u16::from_ne_bytes(value.to_ne_bytes()),
            ]),
            Instruction::Const32 { dst, value } => {
                let bits = value.to_le_bytes();
                output.extend([
                    0x14 | u16::from(integer(*dst)?) << 8,
                    u16::from_le_bytes([bits[0], bits[1]]),
                    u16::from_le_bytes([bits[2], bits[3]]),
                ]);
            }
            Instruction::ConstString { dst, string } => {
                output.extend([0x1a | u16::from(reference(*dst)?) << 8, *string])
            }
            Instruction::Move { dst, src } => {
                output.push(0x01 | u16::from(integer(*dst)?) << 8 | u16::from(integer(*src)?) << 12)
            }
            Instruction::MoveObject { dst, src } => output
                .push(0x07 | u16::from(reference(*dst)?) << 8 | u16::from(reference(*src)?) << 12),
            Instruction::NewInstance { dst, ty } => {
                output.extend([0x22 | u16::from(reference(*dst)?) << 8, *ty])
            }
            Instruction::IGet { dst, object, field }
            | Instruction::IPut {
                src: dst,
                object,
                field,
            } => {
                let opcode = match (instruction, dst.kind) {
                    (Instruction::IGet { .. }, ValueKind::Reference) => 0x54,
                    (Instruction::IGet { .. }, _) => 0x52,
                    (Instruction::IPut { .. }, ValueKind::Reference) => 0x5b,
                    (Instruction::IPut { .. }, _) => 0x59,
                    _ => unreachable!(),
                };
                if object.kind != ValueKind::Reference || dst.index > 15 || object.index > 15 {
                    return Err(DexError::InvalidInput("invalid instance field register"));
                }
                output.extend([
                    opcode | u16::from(dst.index) << 8 | u16::from(object.index) << 12,
                    *field,
                ]);
            }
            Instruction::AddInt { dst, left, right }
            | Instruction::SubInt { dst, left, right }
            | Instruction::MulInt { dst, left, right }
            | Instruction::DivInt { dst, left, right }
            | Instruction::RemInt { dst, left, right } => {
                let opcode = match instruction {
                    Instruction::AddInt { .. } => 0x90,
                    Instruction::SubInt { .. } => 0x91,
                    Instruction::MulInt { .. } => 0x92,
                    Instruction::DivInt { .. } => 0x93,
                    _ => 0x94,
                };
                output.extend([
                    opcode | u16::from(integer(*dst)?) << 8,
                    u16::from(integer(*left)?) | u16::from(integer(*right)?) << 8,
                ]);
            }
            Instruction::RemIntLit8 { dst, src, value } => output.extend([
                0xdc | u16::from(integer(*dst)?) << 8,
                u16::from(integer(*src)?) | u16::from(*value as u8) << 8,
            ]),
            Instruction::AddInt2Addr { dst, src } => {
                output.push(0xb0 | u16::from(integer(*dst)?) << 8 | u16::from(integer(*src)?) << 12)
            }
            Instruction::AddIntLit8 { dst, src, value } => output.extend([
                0xd8 | u16::from(integer(*dst)?) << 8,
                u16::from(integer(*src)?) | u16::from(*value as u8) << 8,
            ]),
            Instruction::IfGe {
                left,
                right,
                target,
            } => output.extend([
                0x35 | u16::from(integer(*left)?) << 8 | u16::from(integer(*right)?) << 12,
                branch(&labels, *target, cursor)?,
            ]),
            Instruction::IfNez { value, target } => output.extend([
                0x39 | u16::from(integer(*value)?) << 8,
                branch(&labels, *target, cursor)?,
            ]),
            Instruction::IfEqz { value, target } => output.extend([
                0x38 | u16::from(integer(*value)?) << 8,
                branch(&labels, *target, cursor)?,
            ]),
            Instruction::IfLt {
                left,
                right,
                target,
            }
            | Instruction::IfLe {
                left,
                right,
                target,
            }
            | Instruction::IfGt {
                left,
                right,
                target,
            } => {
                let opcode = match instruction {
                    Instruction::IfLt { .. } => 0x34,
                    Instruction::IfLe { .. } => 0x37,
                    _ => 0x36,
                };
                output.extend([
                    opcode | u16::from(integer(*left)?) << 8 | u16::from(integer(*right)?) << 12,
                    branch(&labels, *target, cursor)?,
                ]);
            }
            Instruction::IfNe {
                left,
                right,
                target,
            } => {
                if left.kind != right.kind || left.index > 15 || right.index > 15 {
                    return Err(DexError::InvalidInput("if-ne register mismatch"));
                }
                output.extend([
                    0x33 | u16::from(left.index) << 8 | u16::from(right.index) << 12,
                    branch(&labels, *target, cursor)?,
                ]);
            }
            Instruction::InvokeStatic { method, args }
            | Instruction::InvokeVirtual { method, args }
            | Instruction::InvokeSuper { method, args }
            | Instruction::InvokeDirect { method, args } => {
                if args.len() > 5 {
                    return Err(DexError::InvalidInput(
                        "invoke-static exceeds five arguments",
                    ));
                }
                let mut registers = 0_u16;
                for (slot, arg) in args.iter().enumerate() {
                    if arg.index > 15 {
                        return Err(DexError::InvalidInput("M2 register index exceeds 15"));
                    }
                    registers |= u16::from(arg.index) << (slot * 4)
                }
                let opcode = match instruction {
                    Instruction::InvokeVirtual { .. } => 0x6e,
                    Instruction::InvokeSuper { .. } => 0x6f,
                    Instruction::InvokeDirect { .. } => 0x70,
                    _ => 0x71,
                };
                output.extend([
                    opcode | u16::try_from(args.len()).unwrap() << 12,
                    *method,
                    registers,
                ]);
            }
            Instruction::MoveResult { dst } => output.push(0x0a | u16::from(integer(*dst)?) << 8),
            Instruction::MoveResultObject { dst } => {
                output.push(0x0c | u16::from(reference(*dst)?) << 8)
            }
            Instruction::Goto16 { target } => {
                output.extend([0x29, branch(&labels, *target, cursor)?])
            }
            Instruction::Return { value } => output.push(0x0f | u16::from(integer(*value)?) << 8),
            Instruction::ReturnObject { value } => {
                output.push(0x11 | u16::from(reference(*value)?) << 8)
            }
            Instruction::ReturnVoid => output.push(0x0e),
        }
        cursor += width(instruction)
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(index: u8) -> Register {
        Register {
            index,
            kind: ValueKind::I32,
        }
    }
    #[test]
    fn resolves_forward_and_backward_branches() {
        let code = assemble(&[
            Instruction::Const4 {
                dst: r(0),
                value: 0,
            },
            Instruction::Label(Label(1)),
            Instruction::IfGe {
                left: r(1),
                right: r(3),
                target: Label(3),
            },
            Instruction::RemIntLit8 {
                dst: r(2),
                src: r(1),
                value: 2,
            },
            Instruction::IfNez {
                value: r(2),
                target: Label(2),
            },
            Instruction::AddInt2Addr {
                dst: r(0),
                src: r(1),
            },
            Instruction::Goto16 { target: Label(4) },
            Instruction::Label(Label(2)),
            Instruction::AddIntLit8 {
                dst: r(0),
                src: r(0),
                value: 1,
            },
            Instruction::Label(Label(4)),
            Instruction::AddIntLit8 {
                dst: r(1),
                src: r(1),
                value: 1,
            },
            Instruction::Goto16 { target: Label(1) },
            Instruction::Label(Label(3)),
            Instruction::Return { value: r(0) },
        ])
        .unwrap();
        assert_eq!(code[2], 15);
        assert_eq!(code[15], 0xfff3)
    }
    #[test]
    fn rejects_bad_register_kind() {
        let error = assemble(&[Instruction::Return {
            value: Register {
                index: 0,
                kind: ValueKind::Reference,
            },
        }])
        .unwrap_err();
        assert!(matches!(error, DexError::InvalidInput(_)))
    }
    #[test]
    fn rejects_missing_label() {
        assert!(assemble(&[Instruction::Goto16 { target: Label(9) }]).is_err())
    }
    #[test]
    fn encodes_reference_instructions() {
        let a = Register {
            index: 1,
            kind: ValueKind::Reference,
        };
        let b = Register {
            index: 2,
            kind: ValueKind::Reference,
        };
        assert_eq!(
            assemble(&[
                Instruction::ConstString { dst: a, string: 7 },
                Instruction::MoveObject { dst: b, src: a },
                Instruction::NewInstance { dst: a, ty: 9 },
                Instruction::InvokeDirect {
                    method: 4,
                    args: vec![a]
                },
                Instruction::MoveResultObject { dst: b },
                Instruction::ReturnObject { value: b },
            ])
            .unwrap(),
            vec![0x011a, 7, 0x1207, 0x0122, 9, 0x1070, 4, 1, 0x020c, 0x0211]
        );
    }
    #[test]
    fn rejects_scalar_in_object_instruction() {
        assert!(assemble(&[Instruction::ReturnObject { value: r(0) }]).is_err());
    }
}
