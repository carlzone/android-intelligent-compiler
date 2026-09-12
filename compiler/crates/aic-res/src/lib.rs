//! Android-35 binary manifest writer for AIC's resource-free application profile.
use std::fmt::Write;
const NONE: u32 = u32::MAX;
const URI: &str = "http://schemas.android.com/apk/res/android";
// Android 35 android.R constants, independently checked with javap -constants.
const ATTRS: [(&str, u32); 6] = [
    ("theme", 16_842_752),
    ("label", 16_842_753),
    ("name", 16_842_755),
    ("exported", 16_842_768),
    ("minSdkVersion", 16_843_276),
    ("targetSdkVersion", 16_843_376),
];
const THEME: u32 = 16_974_401;
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceError(pub &'static str);
impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ResourceError {}
impl From<std::num::TryFromIntError> for ResourceError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self("Manifest size limit exceeded")
    }
}
#[derive(Clone)]
struct Attribute {
    name: &'static str,
    value: String,
    kind: u8,
    data: u32,
    android: bool,
}
struct Element {
    name: &'static str,
    attrs: Vec<Attribute>,
    children: Vec<Element>,
}
fn attr(name: &'static str, value: &str, kind: u8, data: u32) -> Attribute {
    Attribute {
        name,
        value: value.into(),
        kind,
        data,
        android: name != "package",
    }
}
fn element(name: &'static str, attrs: Vec<Attribute>, children: Vec<Element>) -> Element {
    Element {
        name,
        attrs,
        children,
    }
}
/// Compiler-owned manifest model; accepts values, never arbitrary XML.
pub struct Manifest {
    root: Element,
}
impl Manifest {
    #[must_use]
    pub fn new(package: &str, label: &str, activity: &str) -> Self {
        Self::new_multi(package, label, &[activity.to_owned()])
    }
    #[must_use]
    pub fn new_multi(package: &str, label: &str, activities: &[String]) -> Self {
        let activity_nodes = activities
            .iter()
            .enumerate()
            .map(|(index, activity)| {
                element(
                    "activity",
                    vec![
                        attr("name", &format!(".{activity}"), 3, 0),
                        attr(
                            "exported",
                            if index == 0 { "true" } else { "false" },
                            0x12,
                            NONE,
                        ),
                    ],
                    if index == 0 {
                        vec![element(
                            "intent-filter",
                            vec![],
                            vec![
                                element(
                                    "action",
                                    vec![attr("name", "android.intent.action.MAIN", 3, 0)],
                                    vec![],
                                ),
                                element(
                                    "category",
                                    vec![attr("name", "android.intent.category.LAUNCHER", 3, 0)],
                                    vec![],
                                ),
                            ],
                        )]
                    } else {
                        vec![]
                    },
                )
            })
            .collect();
        Self {
            root: element(
                "manifest",
                vec![attr("package", package, 3, 0)],
                vec![
                    element(
                        "uses-sdk",
                        vec![
                            attr("minSdkVersion", "23", 0x10, 23),
                            attr("targetSdkVersion", "35", 0x10, 35),
                        ],
                        vec![],
                    ),
                    element(
                        "application",
                        vec![
                            attr("label", label, 3, 0),
                            attr(
                                "theme",
                                "@android:style/Theme.Material.Light.NoActionBar",
                                1,
                                THEME,
                            ),
                        ],
                        activity_nodes,
                    ),
                ],
            ),
        }
    }
    #[must_use]
    pub fn text(&self) -> String {
        fn escape(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('\r', "&#13;")
                .replace('\n', "&#10;")
                .replace('\t', "&#9;")
        }
        fn emit(e: &Element, depth: usize, out: &mut String) {
            let pad = "  ".repeat(depth);
            let _ = write!(out, "{pad}<{}", e.name);
            if depth == 0 {
                let _ = write!(out, " xmlns:android=\"{URI}\"");
            }
            for a in &e.attrs {
                let _ = write!(
                    out,
                    " {}{}=\"{}\"",
                    if a.android { "android:" } else { "" },
                    a.name,
                    escape(&a.value)
                );
            }
            if e.children.is_empty() {
                out.push_str(" />\n");
            } else {
                out.push_str(">\n");
                for child in &e.children {
                    emit(child, depth + 1, out);
                }
                let _ = writeln!(out, "{pad}</{}>", e.name);
            }
        }
        let mut out = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n".to_string();
        emit(&self.root, 0, &mut out);
        out
    }
    /// # Errors
    /// Returns a checked format-limit error for oversized strings or chunks.
    pub fn binary(&self) -> Result<Vec<u8>, ResourceError> {
        let mut strings: Vec<String> = ATTRS.iter().map(|(n, _)| (*n).into()).collect();
        intern(&mut strings, "android")?;
        intern(&mut strings, URI)?;
        collect(&self.root, &mut strings)?;
        let mut body = string_pool(&strings)?;
        let mut map = vec![];
        for (_, id) in ATTRS {
            word(&mut map, id);
        }
        body.extend(chunk(0x180, 8, map)?);
        body.extend(namespace(0x100, &strings)?);
        emit_binary(&self.root, &strings, &mut body)?;
        body.extend(namespace(0x101, &strings)?);
        chunk(3, 8, body)
    }
}
fn word(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}
fn short(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn chunk(kind: u16, header: u16, body: Vec<u8>) -> Result<Vec<u8>, ResourceError> {
    let size = chunk_size(body.len())?;
    let mut out = vec![];
    short(&mut out, kind);
    short(&mut out, header);
    word(&mut out, size);
    out.extend(body);
    Ok(out)
}
fn chunk_size(payload: usize) -> Result<u32, ResourceError> {
    Ok(u32::try_from(
        payload
            .checked_add(8)
            .ok_or(ResourceError("Manifest size overflow"))?,
    )?)
}
fn intern(strings: &mut Vec<String>, s: &str) -> Result<(), ResourceError> {
    if !strings.iter().any(|v| v == s) {
        u32::try_from(strings.len())?;
        strings.push(s.into());
    }
    Ok(())
}
fn index(strings: &[String], s: &str) -> Result<u32, ResourceError> {
    u32::try_from(
        strings
            .iter()
            .position(|v| v == s)
            .ok_or(ResourceError("Missing manifest string"))?,
    )
    .map_err(Into::into)
}
fn collect(e: &Element, strings: &mut Vec<String>) -> Result<(), ResourceError> {
    intern(strings, e.name)?;
    for a in &e.attrs {
        if a.value.chars().any(|c| !matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
            return Err(ResourceError("Manifest value contains a character forbidden by XML 1.0"));
        }
        intern(strings, a.name)?;
        if a.kind == 3 {
            intern(strings, &a.value)?;
        }
    }
    for child in &e.children {
        collect(child, strings)?;
    }
    Ok(())
}
fn string_pool(strings: &[String]) -> Result<Vec<u8>, ResourceError> {
    let mut offsets = vec![];
    let mut data = vec![];
    for s in strings {
        word(&mut offsets, u32::try_from(data.len())?);
        let units: Vec<_> = s.encode_utf16().collect();
        let len = u32::try_from(units.len())?;
        if len > 0x7fff_ffff {
            return Err(ResourceError("Manifest string too long"));
        }
        if len > 0x7fff {
            short(&mut data, u16::try_from((len >> 16) | 0x8000)?);
            short(&mut data, u16::try_from(len & 0xffff)?);
        } else {
            short(&mut data, u16::try_from(len)?);
        }
        for unit in units {
            short(&mut data, unit);
        }
        short(&mut data, 0);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let mut body = vec![];
    word(&mut body, u32::try_from(strings.len())?);
    word(&mut body, 0);
    word(&mut body, 0);
    word(
        &mut body,
        u32::try_from(
            28_usize
                .checked_add(offsets.len())
                .ok_or(ResourceError("String pool overflow"))?,
        )?,
    );
    word(&mut body, 0);
    body.extend(offsets);
    body.extend(data);
    chunk(1, 28, body)
}
fn node() -> Vec<u8> {
    let mut out = vec![];
    word(&mut out, 1);
    word(&mut out, NONE);
    out
}
fn namespace(kind: u16, strings: &[String]) -> Result<Vec<u8>, ResourceError> {
    let mut out = node();
    word(&mut out, index(strings, "android")?);
    word(&mut out, index(strings, URI)?);
    chunk(kind, 16, out)
}
fn emit_binary(e: &Element, strings: &[String], out: &mut Vec<u8>) -> Result<(), ResourceError> {
    let mut body = node();
    word(&mut body, NONE);
    word(&mut body, index(strings, e.name)?);
    short(&mut body, 20);
    short(&mut body, 20);
    short(&mut body, u16::try_from(e.attrs.len())?);
    for _ in 0..3 {
        short(&mut body, 0);
    }
    let mut attrs: Vec<_> = e.attrs.iter().collect();
    attrs.sort_by_key(|a| {
        ATTRS
            .iter()
            .find(|(n, _)| *n == a.name)
            .map_or(0, |(_, id)| *id)
    });
    for a in attrs {
        word(
            &mut body,
            if a.android {
                index(strings, URI)?
            } else {
                NONE
            },
        );
        word(&mut body, index(strings, a.name)?);
        let data = if a.kind == 3 {
            index(strings, &a.value)?
        } else {
            a.data
        };
        word(&mut body, if a.kind == 3 { data } else { NONE });
        short(&mut body, 8);
        body.push(0);
        body.push(a.kind);
        word(&mut body, data);
    }
    out.extend(chunk(0x102, 16, body)?);
    for child in &e.children {
        emit_binary(child, strings, out)?;
    }
    let mut end = node();
    word(&mut end, NONE);
    word(&mut end, index(strings, e.name)?);
    out.extend(chunk(0x103, 16, end)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_invalid_xml_and_typed_attributes() {
        assert!(chunk_size(usize::MAX).is_err());
        assert!(chunk_size(u32::MAX as usize).is_err());
        assert!(Manifest::new("dev.aic.test", "bad\0", "Main")
            .binary()
            .is_err());
        assert!(Manifest::new("dev.aic.test", "bad\u{ffff}", "Main")
            .binary()
            .is_err());
        let bytes = Manifest::new("dev.aic.test", "Label", "Main")
            .binary()
            .unwrap();
        let read = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let mut at = 8;
        let mut typed = vec![];
        while at < bytes.len() {
            let kind = u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap());
            if kind == 0x180 {
                let ids: Vec<_> = (at + 8..at + read(at + 4) as usize)
                    .step_by(4)
                    .map(read)
                    .collect();
                assert_eq!(
                    ids,
                    vec![
                        0x0101_0000,
                        0x0101_0001,
                        0x0101_0003,
                        0x0101_0010,
                        0x0101_020c,
                        0x0101_0270
                    ]
                );
            }
            if kind == 0x102 {
                let count = u16::from_le_bytes(bytes[at + 28..at + 30].try_into().unwrap());
                for i in 0..usize::from(count) {
                    let attr = at + 36 + i * 20;
                    assert_eq!(&bytes[attr + 12..attr + 15], &[8, 0, 0]);
                    if bytes[attr + 15] != 3 {
                        typed.push((bytes[attr + 15], read(attr + 16)));
                    }
                }
            }
            at += read(at + 4) as usize;
        }
        assert_eq!(
            typed,
            vec![(0x10, 23), (0x10, 35), (1, 0x0103_0241), (0x12, u32::MAX)]
        );
    }
    #[test]
    fn unicode_boundaries_and_chunk_layout() {
        for label in [
            "\u{7b46}\u{8a18} \u{1f4dd} &\"<>\r\n\t".into(),
            "x".repeat(32767),
            "x".repeat(32768),
            "x".repeat(65536),
        ] {
            let m = Manifest::new("dev.aic.test", &label, "MainActivity");
            let bytes = m.binary().unwrap();
            assert_eq!(bytes, m.binary().unwrap());
            assert_eq!(
                u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
                bytes.len()
            );
            let mut at = 8;
            while at < bytes.len() {
                assert_eq!(at % 4, 0);
                let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
                assert!(size >= 8);
                at += size;
            }
            assert_eq!(at, bytes.len());
        }
        assert!(Manifest::new("dev.aic.test", "&\"<>", "Main")
            .text()
            .contains("&amp;&quot;&lt;&gt;"));
        assert!(index(&[], "missing").is_err());
    }
}
