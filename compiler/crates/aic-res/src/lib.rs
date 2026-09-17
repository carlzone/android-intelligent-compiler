//! Android-35 binary manifest writer and deterministic app-resource foundations.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
const NONE: u32 = u32::MAX;
const URI: &str = "http://schemas.android.com/apk/res/android";
// Android 35 android.R constants, independently checked with javap -constants.
const ATTRS: [(&str, u32); 7] = [
    ("theme", 16_842_752),
    ("label", 16_842_753),
    ("icon", 16_842_754),
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

/// Compiler-owned IDs for the application package. Android app resources use
/// package 0x7f; type 0x01 is reserved for AIC strings in IR 0.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceIds {
    strings: BTreeMap<String, u32>,
    colors: BTreeMap<String, u32>,
    drawables: BTreeMap<String, u32>,
    mipmaps: BTreeMap<String, u32>,
    styles: BTreeMap<String, u32>,
}
impl ResourceIds {
    /// Assign IDs by unique resource name, independent of declaration or archive order.
    /// # Errors
    /// Rejects duplicate names and the 16-bit entry-index limit.
    pub fn strings<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self, ResourceError> {
        let mut sorted = BTreeSet::new();
        for name in names {
            if !sorted.insert(name) {
                return Err(ResourceError("Duplicate string resource name"));
            }
        }
        let mut strings = BTreeMap::new();
        for (index, name) in sorted.into_iter().enumerate() {
            let entry = u16::try_from(index)
                .map_err(|_| ResourceError("String resource entry limit exceeded"))?;
            strings.insert(name.to_owned(), 0x7f01_0000 | u32::from(entry));
        }
        Ok(Self {
            strings,
            colors: BTreeMap::new(),
            drawables: BTreeMap::new(),
            mipmaps: BTreeMap::new(),
            styles: BTreeMap::new(),
        })
    }
    /// Assign IDs for every supported resource type from canonical names.
    /// # Errors
    /// Rejects duplicate names or type entry counts beyond Android's 16-bit limit.
    pub fn all<'a>(
        strings: impl IntoIterator<Item = &'a str>,
        colors: impl IntoIterator<Item = &'a str>,
        drawables: impl IntoIterator<Item = &'a str>,
        mipmaps: impl IntoIterator<Item = &'a str>,
        styles: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, ResourceError> {
        fn assign<'a>(
            names: impl IntoIterator<Item = &'a str>,
            type_id: u32,
        ) -> Result<BTreeMap<String, u32>, ResourceError> {
            let mut names: Vec<_> = names.into_iter().collect();
            names.sort_unstable();
            if names.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(ResourceError("Duplicate resource name"));
            }
            names
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    let entry = u16::try_from(index)
                        .map_err(|_| ResourceError("Resource entry limit exceeded"))?;
                    Ok((
                        name.to_owned(),
                        0x7f00_0000 | type_id << 16 | u32::from(entry),
                    ))
                })
                .collect()
        }
        Ok(Self {
            strings: assign(strings, 1)?,
            colors: assign(colors, 2)?,
            drawables: assign(drawables, 3)?,
            mipmaps: assign(mipmaps, 4)?,
            styles: assign(styles, 5)?,
        })
    }
    #[must_use]
    pub fn string(&self, name: &str) -> Option<u32> {
        self.strings.get(name).copied()
    }
    pub fn iter_strings(&self) -> impl Iterator<Item = (&str, u32)> + '_ {
        self.strings.iter().map(|(name, id)| (name.as_str(), *id))
    }
    #[must_use]
    pub fn color(&self, name: &str) -> Option<u32> {
        self.colors.get(name).copied()
    }
    #[must_use]
    pub fn drawable(&self, name: &str) -> Option<u32> {
        self.drawables.get(name).copied()
    }
    #[must_use]
    pub fn mipmap(&self, name: &str) -> Option<u32> {
        self.mipmaps.get(name).copied()
    }
    #[must_use]
    pub fn style(&self, name: &str) -> Option<u32> {
        self.styles.get(name).copied()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StringValue {
    pub name: String,
    pub locale: Option<String>,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileResource {
    pub name: String,
    pub extension: String,
    pub bytes: Vec<u8>,
    pub launcher: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePackage {
    pub table: Vec<u8>,
    pub entries: Vec<(String, Vec<u8>)>,
    pub ids: ResourceIds,
}

/// Emit AIC's bounded Android application resource table and canonical file entries.
/// # Errors
/// Rejects duplicate resources, unsupported locale configurations, and binary size limits.
pub fn package_resources(
    package_name: &str,
    strings: &[StringValue],
    colors: &[(String, u32)],
    files: &[FileResource],
    theme: Option<(&str, &str, &str)>,
) -> Result<ResourcePackage, ResourceError> {
    let default_strings: Vec<_> = strings
        .iter()
        .filter(|value| value.locale.is_none())
        .map(|value| value.name.as_str())
        .collect();
    let drawables: Vec<_> = files
        .iter()
        .filter(|file| !file.launcher)
        .map(|file| file.name.as_str())
        .collect();
    let mipmaps: Vec<_> = files
        .iter()
        .filter(|file| file.launcher)
        .map(|file| file.name.as_str())
        .collect();
    let styles = theme.iter().map(|(name, _, _)| *name);
    let ids = ResourceIds::all(
        default_strings,
        colors.iter().map(|(name, _)| name.as_str()),
        drawables,
        mipmaps,
        styles,
    )?;
    if strings.is_empty() && colors.is_empty() && files.is_empty() && theme.is_none() {
        return Ok(ResourcePackage {
            table: Vec::new(),
            entries: Vec::new(),
            ids,
        });
    }
    let mut entries: Vec<_> = files
        .iter()
        .map(|file| {
            let directory = if file.launcher { "mipmap" } else { "drawable" };
            (
                format!("res/{directory}/{}.{}", file.name, file.extension),
                file.bytes.clone(),
            )
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    if entries.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(ResourceError("Duplicate packaged resource path"));
    }
    let resolved_theme = theme
        .map(|(name, primary, accent)| -> Result<_, ResourceError> {
            Ok((
                name,
                ids.color(primary)
                    .ok_or(ResourceError("Unknown primary theme color"))?,
                ids.color(accent)
                    .ok_or(ResourceError("Unknown accent theme color"))?,
            ))
        })
        .transpose()?;
    let table = resource_table(package_name, strings, colors, files, resolved_theme, &ids)?;
    Ok(ResourcePackage {
        table,
        entries,
        ids,
    })
}

#[derive(Clone)]
struct TableEntry {
    name: String,
    data_type: u8,
    data: u32,
}

#[allow(clippy::too_many_lines)]
fn resource_table(
    package_name: &str,
    strings: &[StringValue],
    colors: &[(String, u32)],
    files: &[FileResource],
    theme: Option<(&str, u32, u32)>,
    ids: &ResourceIds,
) -> Result<Vec<u8>, ResourceError> {
    let mut values = Vec::new();
    for string in strings {
        intern(&mut values, &string.value)?;
    }
    for file in files {
        let directory = if file.launcher { "mipmap" } else { "drawable" };
        intern(
            &mut values,
            &format!("res/{directory}/{}.{}", file.name, file.extension),
        )?;
    }
    let global_pool = string_pool(&values)?;
    let type_names = ["string", "color", "drawable", "mipmap", "style"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let type_pool = string_pool(&type_names)?;
    let mut keys = BTreeSet::new();
    keys.extend(strings.iter().map(|value| value.name.clone()));
    keys.extend(colors.iter().map(|(name, _)| name.clone()));
    keys.extend(files.iter().map(|file| file.name.clone()));
    keys.extend(theme.iter().map(|(name, _, _)| (*name).to_owned()));
    let keys: Vec<_> = keys.into_iter().collect();
    let key_pool = string_pool(&keys)?;

    let mut package_body = Vec::new();
    package_body.extend(&type_pool);
    package_body.extend(&key_pool);
    let mut grouped_strings: BTreeMap<Option<String>, Vec<TableEntry>> = BTreeMap::new();
    for value in strings {
        grouped_strings
            .entry(value.locale.clone())
            .or_default()
            .push(TableEntry {
                name: value.name.clone(),
                data_type: 0x03,
                data: index(&values, &value.value)?,
            });
    }
    emit_type_group(
        &mut package_body,
        1,
        ids.strings.len(),
        &keys,
        grouped_strings,
    )?;
    emit_type_group(
        &mut package_body,
        2,
        ids.colors.len(),
        &keys,
        BTreeMap::from([(
            None,
            colors
                .iter()
                .map(|(name, color)| TableEntry {
                    name: name.clone(),
                    data_type: color_data_type(*color),
                    data: *color,
                })
                .collect(),
        )]),
    )?;
    for (type_id, launcher) in [(3, false), (4, true)] {
        let entries = files
            .iter()
            .filter(|file| file.launcher == launcher)
            .map(|file| {
                let directory = if launcher { "mipmap" } else { "drawable" };
                let path = format!("res/{directory}/{}.{}", file.name, file.extension);
                Ok(TableEntry {
                    name: file.name.clone(),
                    data_type: 0x03,
                    data: index(&values, &path)?,
                })
            })
            .collect::<Result<Vec<_>, ResourceError>>()?;
        emit_type_group(
            &mut package_body,
            type_id,
            entries.len(),
            &keys,
            BTreeMap::from([(None, entries)]),
        )?;
    }
    if let Some((name, primary, accent)) = theme {
        emit_style_type(&mut package_body, name, primary, accent, &keys)?;
    } else {
        emit_type_group(&mut package_body, 5, 0, &keys, BTreeMap::new())?;
    }

    let package_header_size = 288_u16;
    let mut package = Vec::new();
    short(&mut package, 0x0200);
    short(&mut package, package_header_size);
    word(&mut package, 0);
    word(&mut package, 0x7f);
    let mut name = [0_u16; 128];
    for (slot, unit) in name.iter_mut().zip(package_name.encode_utf16()) {
        *slot = unit;
    }
    for unit in name {
        short(&mut package, unit);
    }
    word(&mut package, u32::from(package_header_size));
    word(&mut package, 5);
    word(
        &mut package,
        u32::from(package_header_size) + u32::try_from(type_pool.len())?,
    );
    word(&mut package, u32::try_from(keys.len())?);
    word(&mut package, 0);
    package.extend(package_body);
    let package_size = u32::try_from(package.len())?;
    package[4..8].copy_from_slice(&package_size.to_le_bytes());

    let mut out = Vec::new();
    short(&mut out, 0x0002);
    short(&mut out, 12);
    word(&mut out, 0);
    word(&mut out, 1);
    out.extend(global_pool);
    out.extend(package);
    let size = u32::try_from(out.len())?;
    out[4..8].copy_from_slice(&size.to_le_bytes());
    Ok(out)
}

fn emit_type_group(
    out: &mut Vec<u8>,
    type_id: u8,
    entry_count: usize,
    keys: &[String],
    configurations: BTreeMap<Option<String>, Vec<TableEntry>>,
) -> Result<(), ResourceError> {
    if entry_count == 0 {
        return Ok(());
    }
    let mut spec = Vec::new();
    short(&mut spec, 0x0202);
    short(&mut spec, 16);
    word(&mut spec, u32::try_from(16 + entry_count * 4)?);
    spec.push(type_id);
    spec.push(0);
    short(&mut spec, 0);
    word(&mut spec, u32::try_from(entry_count)?);
    for _ in 0..entry_count {
        word(&mut spec, 0x4000_0000);
    }
    out.extend(spec);
    let names = configurations
        .values()
        .flatten()
        .map(|entry| entry.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if names.len() != entry_count {
        return Err(ResourceError("Resource type entry count mismatch"));
    }
    for (locale, mut entries) in configurations {
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        out.extend(type_chunk(
            type_id,
            &names,
            keys,
            locale.as_deref(),
            &entries,
        )?);
    }
    Ok(())
}

fn type_chunk(
    type_id: u8,
    names: &[String],
    keys: &[String],
    locale: Option<&str>,
    entries: &[TableEntry],
) -> Result<Vec<u8>, ResourceError> {
    let entry_count = names.len();
    let header_size = 84_u16;
    let entries_start = usize::from(header_size) + entry_count * 4;
    let mut data = Vec::new();
    let by_name: BTreeMap<_, _> = entries.iter().map(|entry| (&entry.name, entry)).collect();
    let mut offsets = Vec::new();
    for name in names {
        let Some(entry) = by_name.get(name) else {
            continue;
        };
        offsets.push((name, u32::try_from(data.len())?));
        short(&mut data, 8);
        short(&mut data, 0);
        word(&mut data, index(keys, name)?);
        short(&mut data, 8);
        data.push(0);
        data.push(entry.data_type);
        word(&mut data, entry.data);
    }
    let mut out = Vec::new();
    short(&mut out, 0x0201);
    short(&mut out, header_size);
    word(&mut out, 0);
    out.push(type_id);
    out.push(0);
    short(&mut out, 0);
    word(&mut out, u32::try_from(entry_count)?);
    word(&mut out, u32::try_from(entries_start)?);
    out.extend(resource_config(locale)?);
    let offset_map: BTreeMap<_, _> = offsets.into_iter().collect();
    for name in names {
        word(&mut out, offset_map.get(name).copied().unwrap_or(u32::MAX));
    }
    out.extend(data);
    let size = u32::try_from(out.len())?;
    out[4..8].copy_from_slice(&size.to_le_bytes());
    Ok(out)
}

fn emit_style_type(
    out: &mut Vec<u8>,
    name: &str,
    primary: u32,
    accent: u32,
    keys: &[String],
) -> Result<(), ResourceError> {
    let mut spec = Vec::new();
    short(&mut spec, 0x0202);
    short(&mut spec, 16);
    word(&mut spec, 20);
    spec.extend([5, 0, 0, 0]);
    word(&mut spec, 1);
    word(&mut spec, 0x4000_0000);
    out.extend(spec);
    let mut entry = Vec::new();
    short(&mut entry, 16);
    short(&mut entry, 1);
    word(&mut entry, index(keys, name)?);
    word(&mut entry, 0x0103_0224);
    word(&mut entry, 2);
    for (attribute, value) in [(0x0101_0433, primary), (0x0101_0435, accent)] {
        word(&mut entry, attribute);
        short(&mut entry, 8);
        entry.extend([0, 1]);
        word(&mut entry, value);
    }
    let mut chunk = Vec::new();
    short(&mut chunk, 0x0201);
    short(&mut chunk, 84);
    word(&mut chunk, u32::try_from(88 + entry.len())?);
    chunk.extend([5, 0, 0, 0]);
    word(&mut chunk, 1);
    word(&mut chunk, 88);
    chunk.extend(resource_config(None)?);
    word(&mut chunk, 0);
    chunk.extend(entry);
    out.extend(chunk);
    Ok(())
}

fn resource_config(locale: Option<&str>) -> Result<Vec<u8>, ResourceError> {
    let mut config = vec![0_u8; 64];
    config[..4].copy_from_slice(&64_u32.to_le_bytes());
    if let Some(locale) = locale {
        let mut parts = locale.split('-');
        let language = parts.next().ok_or(ResourceError("Invalid locale"))?;
        config[8..10].copy_from_slice(&pack_locale_part(language, b'a')?);
        for part in parts {
            if part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_uppercase()) {
                config[10..12].copy_from_slice(part.as_bytes());
            } else if part.len() == 3 && part.bytes().all(|byte| byte.is_ascii_digit()) {
                config[10..12].copy_from_slice(&pack_locale_part(part, b'0')?);
            } else if part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                config[36..40].copy_from_slice(part.as_bytes());
            } else {
                return Err(ResourceError("Unsupported Android locale qualifier"));
            }
        }
    }
    Ok(config)
}

fn pack_locale_part(part: &str, base: u8) -> Result<[u8; 2], ResourceError> {
    let bytes = part.as_bytes();
    if bytes.len() == 2 {
        return Ok([bytes[0], bytes[1]]);
    }
    if bytes.len() != 3 || bytes.iter().any(|byte| *byte < base || *byte > base + 25) {
        return Err(ResourceError("Unsupported Android locale qualifier"));
    }
    let first = bytes[0] - base;
    let second = bytes[1] - base;
    let third = bytes[2] - base;
    Ok([0x80 | (third << 2) | (second >> 3), (second << 5) | first])
}

fn color_data_type(color: u32) -> u8 {
    if color >> 24 == 0xff {
        0x1c
    } else {
        0x1d
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
        Self::new_multi_resources(package, label, activities, None, None)
    }
    #[must_use]
    pub fn new_multi_resources(
        package: &str,
        label: &str,
        activities: &[String],
        theme: Option<u32>,
        icon: Option<u32>,
    ) -> Self {
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
                        {
                            let mut attrs = vec![attr("label", label, 3, 0)];
                            attrs.push(attr(
                                "theme",
                                if theme.is_some() {
                                    "@style/app"
                                } else {
                                    "@android:style/Theme.Material.Light.NoActionBar"
                                },
                                1,
                                theme.unwrap_or(THEME),
                            ));
                            if let Some(icon) = icon {
                                attrs.push(attr("icon", "@mipmap/launcher", 1, icon));
                            }
                            attrs
                        },
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
    fn application_string_ids_are_stable_and_order_independent() {
        let first = ResourceIds::strings(["greeting", "action_save"]).unwrap();
        let second = ResourceIds::strings(["action_save", "greeting"]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.string("action_save"), Some(0x7f01_0000));
        assert_eq!(first.string("greeting"), Some(0x7f01_0001));
        assert_eq!(first.string("missing"), None);
        assert_eq!(
            first.iter_strings().collect::<Vec<_>>(),
            vec![("action_save", 0x7f01_0000), ("greeting", 0x7f01_0001)]
        );
        assert_eq!(
            ResourceIds::strings(["same", "same"]).unwrap_err(),
            ResourceError("Duplicate string resource name")
        );
    }
    #[test]
    fn bounded_resource_package_is_reproducible() {
        let strings = vec![
            StringValue {
                name: "greeting".into(),
                locale: None,
                value: "Hello".into(),
            },
            StringValue {
                name: "greeting".into(),
                locale: Some("zh-TW".into()),
                value: "你好".into(),
            },
            StringValue {
                name: "greeting".into(),
                locale: Some("es-419".into()),
                value: "Hola".into(),
            },
        ];
        let colors = vec![
            ("accent".into(), 0xff44_5566),
            ("primary".into(), 0xff11_2233),
        ];
        let files = vec![FileResource {
            name: "launcher".into(),
            extension: "png".into(),
            bytes: vec![1, 2, 3],
            launcher: true,
        }];
        let first = package_resources(
            "dev.aic.resources",
            &strings,
            &colors,
            &files,
            Some(("app", "primary", "accent")),
        )
        .unwrap();
        let second = package_resources(
            "dev.aic.resources",
            &strings,
            &colors,
            &files,
            Some(("app", "primary", "accent")),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.ids.string("greeting"), Some(0x7f01_0000));
        assert_eq!(first.ids.color("accent"), Some(0x7f02_0000));
        assert_eq!(first.ids.mipmap("launcher"), Some(0x7f04_0000));
        assert_eq!(first.ids.style("app"), Some(0x7f05_0000));
        assert_eq!(&first.table[..2], &0x0002_u16.to_le_bytes());
    }
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
                        0x0101_0002,
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
