use std::collections::{BTreeMap, BTreeSet};

use aic_ir::{Operation, Program};

use crate::{
    encoding::{encode_mutf8, encode_uleb128, ByteWriter, DexError},
    integrity::{adler32, sha1},
};

const HEADER_SIZE: u32 = 0x70;
const NO_INDEX: u32 = u32::MAX;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Proto {
    ret: &'static str,
    params: Vec<&'static str>,
}
#[derive(Clone, Debug)]
struct Method {
    class: String,
    name: &'static str,
    proto: Proto,
}
#[derive(Clone, Copy)]
struct Section {
    kind: u16,
    count: u32,
    offset: u32,
}

struct Pool {
    strings: Vec<String>,
    string_index: BTreeMap<String, u32>,
    types: Vec<u32>,
    type_index: BTreeMap<String, u16>,
    protos: Vec<Proto>,
    methods: Vec<Method>,
}

impl Pool {
    #[allow(clippy::too_many_lines)]
    fn build(class: &str, text: &str) -> Result<Self, DexError> {
        let protos = vec![
            Proto {
                ret: "V",
                params: vec![],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/os/Bundle;"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/content/Context;"],
            },
            Proto {
                ret: "V",
                params: vec!["I"],
            },
            Proto {
                ret: "V",
                params: vec!["Ljava/lang/CharSequence;"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/view/View;"],
            },
        ];
        let methods = vec![
            Method {
                class: class.to_owned(),
                name: "<init>",
                proto: protos[0].clone(),
            },
            Method {
                class: class.to_owned(),
                name: "onCreate",
                proto: protos[1].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "<init>",
                proto: protos[0].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "onCreate",
                proto: protos[1].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "setContentView",
                proto: protos[5].clone(),
            },
            Method {
                class: "Landroid/widget/LinearLayout;".into(),
                name: "<init>",
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/LinearLayout;".into(),
                name: "setOrientation",
                proto: protos[3].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "<init>",
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "setText",
                proto: protos[4].clone(),
            },
            Method {
                class: "Landroid/view/ViewGroup;".into(),
                name: "addView",
                proto: protos[5].clone(),
            },
        ];
        let mut strings = BTreeSet::new();
        strings.insert(text.to_owned());
        for value in [
            class,
            "Landroid/app/Activity;",
            "Landroid/content/Context;",
            "Landroid/os/Bundle;",
            "Landroid/view/View;",
            "Landroid/view/ViewGroup;",
            "Landroid/widget/LinearLayout;",
            "Landroid/widget/TextView;",
            "Ljava/lang/CharSequence;",
            "I",
            "V",
            "<init>",
            "onCreate",
            "setContentView",
            "setOrientation",
            "setText",
            "addView",
        ] {
            strings.insert(value.to_owned());
        }
        for proto in &protos {
            strings.insert(shorty(proto));
        }
        let mut strings: Vec<_> = strings.into_iter().collect();
        strings.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        let string_index: BTreeMap<String, u32> = strings
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Ok((
                    s.clone(),
                    u32::try_from(i).map_err(|_| DexError::IndexOverflow("string"))?,
                ))
            })
            .collect::<Result<_, DexError>>()?;
        let mut descriptors = BTreeSet::new();
        for method in &methods {
            descriptors.insert(method.class.clone());
            descriptors.insert(method.proto.ret.into());
            for p in &method.proto.params {
                descriptors.insert((*p).into());
            }
        }
        let mut types: Vec<u32> = descriptors.iter().map(|d| string_index[d]).collect();
        types.sort_unstable();
        let type_index: BTreeMap<String, u16> = types
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let descriptor = strings[usize::try_from(*s).unwrap()].clone();
                Ok((
                    descriptor,
                    u16::try_from(i).map_err(|_| DexError::IndexOverflow("type"))?,
                ))
            })
            .collect::<Result<_, DexError>>()?;
        let mut protos = protos;
        protos.sort_by_key(|p| {
            (
                type_index[p.ret],
                p.params.iter().map(|v| type_index[*v]).collect::<Vec<_>>(),
            )
        });
        protos.dedup();
        let proto_index = |p: &Proto| {
            protos
                .iter()
                .position(|candidate| candidate == p)
                .ok_or(DexError::InvalidInput("missing prototype"))
        };
        let mut methods = methods;
        methods.sort_by_key(|m| {
            (
                type_index[&m.class],
                string_index[m.name],
                proto_index(&m.proto).unwrap(),
            )
        });
        Ok(Self {
            strings,
            string_index,
            types,
            type_index,
            protos,
            methods,
        })
    }
    fn method(&self, class: &str, name: &str, proto: &Proto) -> Result<u16, DexError> {
        self.methods
            .iter()
            .position(|m| m.class == class && m.name == name && &m.proto == proto)
            .ok_or(DexError::InvalidInput("missing method"))
            .and_then(|i| u16::try_from(i).map_err(|_| DexError::IndexOverflow("method")))
    }
    fn proto_index(&self, proto: &Proto) -> Result<u16, DexError> {
        self.protos
            .iter()
            .position(|p| p == proto)
            .ok_or(DexError::InvalidInput("missing prototype"))
            .and_then(|i| u16::try_from(i).map_err(|_| DexError::IndexOverflow("prototype")))
    }
}

fn shorty(proto: &Proto) -> String {
    let mut s = String::from(shorty_type(proto.ret));
    for p in &proto.params {
        s.push_str(shorty_type(p));
    }
    s
}
fn shorty_type(value: &str) -> &str {
    if value.starts_with('L') || value.starts_with('[') {
        "L"
    } else {
        value
    }
}
fn u32_len(value: usize) -> Result<u32, DexError> {
    u32::try_from(value).map_err(|_| DexError::ArithmeticOverflow)
}
fn align4(value: u32) -> Result<u32, DexError> {
    value
        .checked_add(3)
        .map(|v| v & !3)
        .ok_or(DexError::ArithmeticOverflow)
}

/// Lowers a verified M1 program to a deterministic DEX 035 Activity.
///
/// # Errors
/// Returns a typed DEX error if the program is outside the M1 lowering shape or
/// if any index, offset, or encoded size cannot be represented safely.
pub fn encode_activity_dex(program: &Program) -> Result<Vec<u8>, DexError> {
    let text =
        match program.activity.on_create.as_slice() {
            [Operation::LinearLayout { id: root, .. }, Operation::TextView {
                id: message, text, ..
            }, Operation::AddView { parent, child, .. }, Operation::SetContentView { view, .. }]
                if parent == root && child == message && view == root =>
            {
                text
            }
            _ => return Err(DexError::InvalidInput(
                "M1 lowering requires linear layout, text view, add_view, then set_content_view",
            )),
        };
    let class = format!(
        "L{}/{};",
        program.package.replace('.', "/"),
        program.activity.name
    );
    let pool = Pool::build(&class, text)?;
    encode(&pool, &class, text)
}

#[allow(clippy::too_many_lines)]
fn encode(pool: &Pool, class: &str, text: &str) -> Result<Vec<u8>, DexError> {
    let p0 = Proto {
        ret: "V",
        params: vec![],
    };
    let pb = Proto {
        ret: "V",
        params: vec!["Landroid/os/Bundle;"],
    };
    let pc = Proto {
        ret: "V",
        params: vec!["Landroid/content/Context;"],
    };
    let pi = Proto {
        ret: "V",
        params: vec!["I"],
    };
    let ps = Proto {
        ret: "V",
        params: vec!["Ljava/lang/CharSequence;"],
    };
    let pv = Proto {
        ret: "V",
        params: vec!["Landroid/view/View;"],
    };
    let own_init = pool.method(class, "<init>", &p0)?;
    let own_create = pool.method(class, "onCreate", &pb)?;
    let ctor = vec![
        0x1070,
        pool.method("Landroid/app/Activity;", "<init>", &p0)?,
        0,
        0x000e,
    ];
    let create = vec![
        0x206f,
        pool.method("Landroid/app/Activity;", "onCreate", &pb)?,
        0x0043,
        0x0022,
        pool.type_index["Landroid/widget/LinearLayout;"],
        0x2070,
        pool.method("Landroid/widget/LinearLayout;", "<init>", &pc)?,
        0x0030,
        0x1112,
        0x206e,
        pool.method("Landroid/widget/LinearLayout;", "setOrientation", &pi)?,
        0x0010,
        0x0122,
        pool.type_index["Landroid/widget/TextView;"],
        0x2070,
        pool.method("Landroid/widget/TextView;", "<init>", &pc)?,
        0x0031,
        0x021a,
        u16::try_from(pool.string_index[text]).map_err(|_| DexError::IndexOverflow("string"))?,
        0x206e,
        pool.method("Landroid/widget/TextView;", "setText", &ps)?,
        0x0021,
        0x206e,
        pool.method("Landroid/view/ViewGroup;", "addView", &pv)?,
        0x0010,
        0x206e,
        pool.method("Landroid/app/Activity;", "setContentView", &pv)?,
        0x0003,
        0x000e,
    ];

    let string_ids = HEADER_SIZE;
    let type_ids = string_ids + u32_len(pool.strings.len())? * 4;
    let proto_ids = type_ids + u32_len(pool.types.len())? * 4;
    let method_ids = proto_ids + u32_len(pool.protos.len())? * 12;
    let class_defs = method_ids + u32_len(pool.methods.len())? * 8;
    let data = class_defs + 32;
    let mut cursor = data;
    let mut type_lists = Vec::new();
    for proto in &pool.protos {
        if !proto.params.is_empty() {
            cursor = align4(cursor)?;
            let off = cursor;
            cursor += 4 + u32_len(proto.params.len())? * 2;
            if proto.params.len() % 2 != 0 {
                cursor += 2;
            }
            type_lists.push((proto.clone(), off));
        }
    }
    let string_data = cursor;
    let mut string_offsets = Vec::new();
    for value in &pool.strings {
        string_offsets.push(cursor);
        cursor += u32_len(
            encode_uleb128(u32_len(value.encode_utf16().count())?).len()
                + encode_mutf8(value).len(),
        )?;
    }
    cursor = align4(cursor)?;
    let ctor_code = cursor;
    cursor += 16 + u32_len(ctor.len())? * 2;
    cursor = align4(cursor)?;
    let create_code = cursor;
    cursor += 16 + u32_len(create.len())? * 2;
    let class_data = cursor;
    let class_data_bytes = class_data_item(own_init, ctor_code, own_create, create_code);
    cursor += u32_len(class_data_bytes.len())?;
    cursor = align4(cursor)?;
    let map = cursor;
    let map_count = 11_u32;
    let file_size = map + 4 + map_count * 12;

    let mut out = ByteWriter::new();
    out.write_bytes(b"dex\n035\0");
    out.write_u32(0);
    out.write_bytes(&[0; 20]);
    out.write_u32(file_size);
    out.write_u32(HEADER_SIZE);
    out.write_u32(0x1234_5678);
    out.write_u32(0);
    out.write_u32(0);
    out.write_u32(map);
    size_off(&mut out, pool.strings.len(), string_ids)?;
    size_off(&mut out, pool.types.len(), type_ids)?;
    size_off(&mut out, pool.protos.len(), proto_ids)?;
    size_off(&mut out, 0, 0)?;
    size_off(&mut out, pool.methods.len(), method_ids)?;
    size_off(&mut out, 1, class_defs)?;
    out.write_u32(file_size - data);
    out.write_u32(data);
    for offset in &string_offsets {
        out.write_u32(*offset);
    }
    for value in &pool.types {
        out.write_u32(*value);
    }
    for proto in &pool.protos {
        out.write_u32(pool.string_index[&shorty(proto)]);
        out.write_u32(u32::from(pool.type_index[proto.ret]));
        out.write_u32(
            type_lists
                .iter()
                .find(|(p, _)| p == proto)
                .map_or(0, |v| v.1),
        );
    }
    for method in &pool.methods {
        out.write_u16(pool.type_index[&method.class]);
        out.write_u16(pool.proto_index(&method.proto)?);
        out.write_u32(pool.string_index[method.name]);
    }
    out.write_u32(u32::from(pool.type_index[class]));
    out.write_u32(1);
    out.write_u32(u32::from(pool.type_index["Landroid/app/Activity;"]));
    out.write_u32(0);
    out.write_u32(NO_INDEX);
    out.write_u32(0);
    out.write_u32(class_data);
    out.write_u32(0);
    for (proto, expected) in &type_lists {
        out.align(4)?;
        debug_assert_eq!(u32_len(out.position())?, *expected);
        out.write_u32(u32_len(proto.params.len())?);
        for p in &proto.params {
            out.write_u16(pool.type_index[*p]);
        }
        if proto.params.len() % 2 != 0 {
            out.write_u16(0);
        }
    }
    for (value, expected) in pool.strings.iter().zip(&string_offsets) {
        debug_assert_eq!(u32_len(out.position())?, *expected);
        out.write_bytes(&encode_uleb128(u32_len(value.encode_utf16().count())?));
        out.write_bytes(&encode_mutf8(value));
    }
    out.align(4)?;
    write_code(&mut out, 1, 1, 1, &ctor);
    out.align(4)?;
    write_code(&mut out, 5, 2, 2, &create);
    out.write_bytes(&class_data_bytes);
    out.align(4)?;
    let sections = [
        Section {
            kind: 0x0000,
            count: 1,
            offset: 0,
        },
        Section {
            kind: 0x0001,
            count: u32_len(pool.strings.len())?,
            offset: string_ids,
        },
        Section {
            kind: 0x0002,
            count: u32_len(pool.types.len())?,
            offset: type_ids,
        },
        Section {
            kind: 0x0003,
            count: u32_len(pool.protos.len())?,
            offset: proto_ids,
        },
        Section {
            kind: 0x0005,
            count: u32_len(pool.methods.len())?,
            offset: method_ids,
        },
        Section {
            kind: 0x0006,
            count: 1,
            offset: class_defs,
        },
        Section {
            kind: 0x1001,
            count: u32_len(type_lists.len())?,
            offset: type_lists[0].1,
        },
        Section {
            kind: 0x2002,
            count: u32_len(pool.strings.len())?,
            offset: string_data,
        },
        Section {
            kind: 0x2001,
            count: 2,
            offset: ctor_code,
        },
        Section {
            kind: 0x2000,
            count: 1,
            offset: class_data,
        },
        Section {
            kind: 0x1000,
            count: 1,
            offset: map,
        },
    ];
    out.write_u32(map_count);
    for s in sections {
        out.write_u16(s.kind);
        out.write_u16(0);
        out.write_u32(s.count);
        out.write_u32(s.offset);
    }
    debug_assert_eq!(u32_len(out.position())?, file_size);
    let signature = sha1(&out.bytes()[32..]);
    out.bytes[12..32].copy_from_slice(&signature);
    let checksum = adler32(&out.bytes()[12..]);
    out.patch_u32(8, checksum)?;
    Ok(out.into_bytes())
}

fn class_data_item(init: u16, init_code: u32, create: u16, create_code: u32) -> Vec<u8> {
    let mut v = vec![0, 0, 1, 1];
    v.extend(encode_uleb128(u32::from(init)));
    v.extend(encode_uleb128(0x1_0001));
    v.extend(encode_uleb128(init_code));
    v.extend(encode_uleb128(u32::from(create)));
    v.extend(encode_uleb128(0x4));
    v.extend(encode_uleb128(create_code));
    v
}
fn write_code(out: &mut ByteWriter, registers: u16, ins: u16, outs: u16, code: &[u16]) {
    out.write_u16(registers);
    out.write_u16(ins);
    out.write_u16(outs);
    out.write_u16(0);
    out.write_u32(0);
    out.write_u32(u32::try_from(code.len()).unwrap());
    for word in code {
        out.write_u16(*word);
    }
}
fn size_off(out: &mut ByteWriter, size: usize, offset: u32) -> Result<(), DexError> {
    out.write_u32(u32_len(size)?);
    out.write_u32(if size == 0 { 0 } else { offset });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_ir::parse_program;
    const IR: &str = "aic_version 0.1\napp \"AIC Hello\" package \"dev.aic.generated.hello\" {\nactivity MainActivity {\non_create {\nlet root = android.linear_layout(orientation: vertical)\nlet message = android.text_view(text: \"Hello from AndroidIntelligentCompiler\")\nandroid.add_view(parent: root, child: message)\nandroid.set_content_view(root)\n}\n}\n}\n";
    #[test]
    fn deterministic_activity_dex() {
        let p = parse_program(IR).unwrap();
        let a = encode_activity_dex(&p).unwrap();
        assert_eq!(a, encode_activity_dex(&p).unwrap());
        assert!(a
            .windows(37)
            .any(|w| w == b"Hello from AndroidIntelligentCompiler"));
    }
}
