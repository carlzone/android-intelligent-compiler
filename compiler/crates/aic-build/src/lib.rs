use aic_ir::{parse_program, SourceSpan};
use aic_opt::{optimize_with_report, CompilerOptions, OptimizationLevel};
use std::error::Error;

#[derive(Debug)]
pub struct BuildError {
    pub stage: &'static str,
    pub code: &'static str,
    pub message: String,
    pub location: Option<SourceSpan>,
}
impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)?;
        if let Some(s) = self.location {
            write!(f, " at {}:{}", s.start.line, s.start.column)?;
        }
        write!(f, ": {}", self.message)
    }
}
impl Error for BuildError {}

#[derive(Debug, PartialEq, Eq)]
pub struct BuildArtifacts {
    pub dex: Vec<u8>,
    pub dex_files: Vec<(String, Vec<u8>)>,
    pub manifest: String,
    pub binary_manifest: Vec<u8>,
    pub unsigned_apk: Vec<u8>,
    pub build_profile: String,
    pub package: String,
    pub activity: String,
    pub activities: Vec<String>,
    pub ir_version: String,
    pub catalog_version: &'static str,
    /// Canonical application string IDs, sorted by resource name.
    pub string_resource_ids: Vec<(String, u32)>,
    pub resources_arsc: Vec<u8>,
    pub resource_entries: Vec<(String, Vec<u8>)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAsset {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Compile validated AIC source without filesystem or Android host dependencies.
/// # Errors
/// Returns a source-located diagnostic or a typed DEX lowering failure.
pub fn compile_source(
    source: &str,
    options: CompilerOptions,
) -> Result<BuildArtifacts, BuildError> {
    compile_source_with_assets(source, options, &[])
}

/// Compile validated AIC source with an explicit, bounded project-asset collection.
/// # Errors
/// Returns source-located validation, lowering, resource, asset, or APK packaging failures.
#[allow(clippy::too_many_lines)]
pub fn compile_source_with_assets(
    source: &str,
    options: CompilerOptions,
    assets: &[ProjectAsset],
) -> Result<BuildArtifacts, BuildError> {
    let parsed = parse_program(source).map_err(|e| BuildError {
        stage: "validate",
        code: e.code,
        message: e.message,
        location: e.location,
    })?;
    let (program, optimization) = optimize_with_report(parsed, options);
    let mut dex_files = Vec::new();
    for (index, activity) in program.activities.iter().enumerate() {
        let mut unit = program.clone();
        unit.activity = activity.clone();
        unit.activities = vec![activity.clone()];
        let bytes = aic_dex::encode_activity_dex(&unit).map_err(|e| BuildError {
            stage: "lower",
            code: "AIC6006",
            message: e.to_string(),
            location: None,
        })?;
        dex_files.push((
            if index == 0 {
                "classes.dex".into()
            } else {
                format!("classes{}.dex", index + 1)
            },
            bytes,
        ));
    }
    let dex = dex_files[0].1.clone();
    let build_profile = format!(
        "profile=android-35\ndex=035\nminSdk=23\ntargetSdk=35\noptLevel={}\ncapabilities={}\npermissions=\nfunctions.before={}\nfunctions.after={}\nfunctions.removed={}\nresources.before={}\nresources.after={}\nresources.removed={}\n",
        i32::from(options.optimization_level != OptimizationLevel::None),
        program.capabilities.iter().map(|c| c.name()).collect::<Vec<_>>().join(","),
        optimization.functions_before, optimization.functions_after, optimization.removed_functions(),
        optimization.preferences_before + optimization.tables_before,
        optimization.preferences_after + optimization.tables_after, optimization.removed_resources());
    let activity_names: Vec<_> = program.activities.iter().map(|a| a.name.clone()).collect();
    let files = validate_project_assets(&program, assets)?;
    let strings = program
        .string_resources
        .iter()
        .map(|resource| aic_res::StringValue {
            name: resource.name.clone(),
            locale: resource.locale.clone(),
            value: resource.value.clone(),
        })
        .collect::<Vec<_>>();
    let colors = program
        .color_resources
        .iter()
        .map(|resource| {
            let digits = resource.value.trim_start_matches('#');
            let value = u32::from_str_radix(digits, 16).map_err(|_| BuildError {
                stage: "package",
                code: "AIC8003",
                message: "Verified color could not be encoded".into(),
                location: Some(resource.span),
            })?;
            Ok((
                resource.name.clone(),
                if digits.len() == 6 {
                    0xff00_0000 | value
                } else {
                    value
                },
            ))
        })
        .collect::<Result<Vec<_>, BuildError>>()?;
    let theme = program.app_theme.as_ref().map(|theme| {
        (
            theme.name.as_str(),
            theme.primary.name.as_str(),
            theme.accent.name.as_str(),
        )
    });
    let resources = aic_res::package_resources(&program.package, &strings, &colors, &files, theme)
        .map_err(|e| BuildError {
            stage: "package",
            code: "AIC8003",
            message: e.to_string(),
            location: None,
        })?;
    let theme_id = program
        .app_theme
        .as_ref()
        .and_then(|theme| resources.ids.style(&theme.name));
    let icon_id = program
        .launcher_icon
        .as_ref()
        .and_then(|icon| resources.ids.mipmap(&icon.name));
    let manifest = aic_res::Manifest::new_multi_resources(
        &program.package,
        &program.label,
        &activity_names,
        theme_id,
        icon_id,
    );
    let binary_manifest = manifest.binary().map_err(|e| BuildError {
        stage: "package",
        code: "AIC8001",
        message: e.to_string(),
        location: None,
    })?;
    let unsigned_apk = assemble_apk_resources(
        &binary_manifest,
        &resources.table,
        &resources.entries,
        &dex_files,
    )
    .map_err(|e| BuildError {
        stage: "package",
        code: "AIC8002",
        message: e.to_string(),
        location: None,
    })?;
    Ok(BuildArtifacts {
        dex,
        dex_files,
        manifest: manifest.text(),
        binary_manifest,
        unsigned_apk,
        build_profile,
        package: program.package.clone(),
        activity: format!("{}.{}", program.package, program.activity.name),
        activities: program
            .activities
            .iter()
            .map(|a| format!("{}.{}", program.package, a.name))
            .collect(),
        ir_version: program.version.clone(),
        catalog_version: "aic.capabilities/0.2",
        string_resource_ids: resources
            .ids
            .iter_strings()
            .map(|(name, id)| (name.to_owned(), id))
            .collect(),
        resources_arsc: resources.table,
        resource_entries: resources.entries,
    })
}

fn validate_project_assets(
    program: &aic_ir::Program,
    assets: &[ProjectAsset],
) -> Result<Vec<aic_res::FileResource>, BuildError> {
    const MAX_TOTAL: usize = 10 * 1024 * 1024;
    if assets.len() > 64 || assets.iter().map(|asset| asset.bytes.len()).sum::<usize>() > MAX_TOTAL
    {
        return Err(BuildError {
            stage: "validate",
            code: "AIC8004",
            message: "Project image count or total size exceeds the supported limit".into(),
            location: None,
        });
    }
    let mut supplied = std::collections::BTreeMap::new();
    for asset in assets {
        if supplied
            .insert(asset.name.as_str(), asset.bytes.as_slice())
            .is_some()
        {
            return Err(BuildError {
                stage: "validate",
                code: "AIC8005",
                message: "Duplicate project asset".into(),
                location: None,
            });
        }
    }
    if supplied.len() != program.image_resources.len() {
        return Err(BuildError {
            stage: "validate",
            code: "AIC8006",
            message: "Declared and supplied project images must match exactly".into(),
            location: None,
        });
    }
    let launcher = program
        .launcher_icon
        .as_ref()
        .map(|icon| icon.name.as_str());
    program
        .image_resources
        .iter()
        .map(|image| {
            let bytes = supplied
                .get(image.project_asset.as_str())
                .ok_or_else(|| BuildError {
                    stage: "validate",
                    code: "AIC8006",
                    message: format!("Missing project asset `{}`", image.project_asset),
                    location: Some(image.span),
                })?;
            validate_image(&image.project_asset, bytes).map_err(|message| BuildError {
                stage: "validate",
                code: "AIC8007",
                message,
                location: Some(image.span),
            })?;
            let extension = image
                .project_asset
                .rsplit_once('.')
                .expect("verified asset name")
                .1;
            Ok(aic_res::FileResource {
                name: image.name.clone(),
                extension: extension.into(),
                bytes: bytes.to_vec(),
                launcher: launcher == Some(image.name.as_str()),
            })
        })
        .collect()
}

fn validate_image(name: &str, bytes: &[u8]) -> Result<(), String> {
    const MAX_IMAGE: usize = 4 * 1024 * 1024;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE {
        return Err(format!("Image asset `{name}` exceeds the supported size"));
    }
    let extension = std::path::Path::new(name).extension();
    let png = extension.is_some_and(|value| value.eq_ignore_ascii_case("png"))
        && bytes.len() >= 24
        && bytes[..8] == [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        && &bytes[12..16] == b"IHDR"
        && (1..=8192).contains(&u32::from_be_bytes(
            bytes[16..20].try_into().expect("four bytes"),
        ))
        && (1..=8192).contains(&u32::from_be_bytes(
            bytes[20..24].try_into().expect("four bytes"),
        ));
    let webp = extension.is_some_and(|value| value.eq_ignore_ascii_case("webp"))
        && bytes.len() >= 16
        && &bytes[..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
        && u64::from(u32::from_le_bytes(
            bytes[4..8].try_into().expect("four bytes"),
        )) + 8
            == bytes.len() as u64
        && matches!(&bytes[12..16], b"VP8 " | b"VP8L" | b"VP8X");
    if png || webp {
        Ok(())
    } else {
        Err(format!("Image asset `{name}` has invalid content"))
    }
}

/// Assemble the supported resource-free APK in deterministic entry order.
/// # Errors
/// Returns a ZIP format-limit error if an artifact is oversized.
pub fn assemble_apk(manifest: &[u8], dex: &[u8]) -> Result<Vec<u8>, ZipError> {
    assemble_apk_files(manifest, &[("classes.dex".into(), dex.to_vec())])
}

/// Assemble a deterministic APK containing one or more DEX units.
///
/// # Errors
/// Returns a checked ZIP-format error for invalid names, duplicates, or size limits.
pub fn assemble_apk_files(
    manifest: &[u8],
    dex_files: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, ZipError> {
    let mut empty = vec![0x50, 0x4b, 0x05, 0x06];
    empty.resize(22, 0);
    let mut apk = inject_stored_zip(&empty, "AndroidManifest.xml", manifest)?;
    for (name, bytes) in dex_files {
        apk = inject_stored_zip(&apk, name, bytes)?;
    }
    Ok(apk)
}

/// Assemble deterministic compiler-owned resource and DEX entries.
/// # Errors
/// Rejects unsafe names, duplicate entries, malformed base ZIP state, and ZIP size limits.
pub fn assemble_apk_resources(
    manifest: &[u8],
    resources: &[u8],
    resource_entries: &[(String, Vec<u8>)],
    dex_files: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, ZipError> {
    let mut empty = vec![0x50, 0x4b, 0x05, 0x06];
    empty.resize(22, 0);
    let mut apk = inject_stored_zip(&empty, "AndroidManifest.xml", manifest)?;
    if !resources.is_empty() {
        apk = inject_stored_zip(&apk, "resources.arsc", resources)?;
    }
    for (name, bytes) in resource_entries {
        if !valid_apk_entry(name) || !name.starts_with("res/") {
            return Err(ZipError("Invalid resource ZIP entry name"));
        }
        apk = inject_stored_zip(&apk, name, bytes)?;
    }
    for (name, bytes) in dex_files {
        apk = inject_stored_zip(&apk, name, bytes)?;
    }
    Ok(apk)
}

fn valid_apk_entry(name: &str) -> bool {
    !name.is_empty()
        && name.is_ascii()
        && !name.starts_with('/')
        && !name.contains('\\')
        && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

#[derive(Debug, PartialEq, Eq)]
pub struct ZipError(pub &'static str);
impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for ZipError {}
impl From<std::num::TryFromIntError> for ZipError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self("ZIP size limit exceeded")
    }
}
fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, ZipError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(ZipError("Truncated ZIP"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, ZipError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(ZipError("Truncated ZIP"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
/// Add an aligned, stored DEX entry to an unsigned AAPT2 ZIP.
/// # Errors
/// Rejects malformed, multi-disk, ZIP64, duplicate-entry, or oversized archives.
pub fn inject_stored_zip(base: &[u8], name: &str, data: &[u8]) -> Result<Vec<u8>, ZipError> {
    let eocd = (base.len().saturating_sub(65557)..base.len().saturating_sub(21))
        .rev()
        .find(|&at| {
            u32_at(base, at) == Ok(0x0605_4b50)
                && u16_at(base, at + 20).is_ok_and(|n| at + 22 + usize::from(n) == base.len())
        })
        .ok_or(ZipError("Missing ZIP end record"))?;
    let entries = u16_at(base, eocd + 10)?;
    let count = entries
        .checked_add(1)
        .filter(|n| *n < u16::MAX)
        .ok_or(ZipError("ZIP entry limit exceeded"))?;
    if u16_at(base, eocd + 4)? != 0
        || u16_at(base, eocd + 6)? != 0
        || u16_at(base, eocd + 8)? != entries
    {
        return Err(ZipError("Multi-disk ZIP is unsupported"));
    }
    let central_size = usize::try_from(u32_at(base, eocd + 12)?)?;
    let central_offset = usize::try_from(u32_at(base, eocd + 16)?)?;
    if central_offset.checked_add(central_size) != Some(eocd) {
        return Err(ZipError("Invalid ZIP central directory"));
    }
    let mut cursor = central_offset;
    for _ in 0..entries {
        if u32_at(base, cursor)? != 0x0201_4b50 {
            return Err(ZipError("Invalid ZIP entry"));
        }
        let len = usize::from(u16_at(base, cursor + 28)?);
        let extra = usize::from(u16_at(base, cursor + 30)?);
        let comment = usize::from(u16_at(base, cursor + 32)?);
        let end = cursor
            .checked_add(46 + len + extra + comment)
            .ok_or(ZipError("ZIP overflow"))?;
        if end > eocd {
            return Err(ZipError("Truncated ZIP entry"));
        }
        if &base[cursor + 46..cursor + 46 + len] == name.as_bytes() {
            return Err(ZipError("Duplicate ZIP entry"));
        }
        cursor = end;
    }
    if cursor != eocd {
        return Err(ZipError("ZIP entry count mismatch"));
    }
    let mut out = base[..central_offset].to_vec();
    let local_offset = u32::try_from(out.len())?;
    let crc = crc32(data);
    let size = u32::try_from(data.len())?;
    let name_bytes = name.as_bytes();
    let name_len = u16::try_from(name_bytes.len())?;
    // A valid private ZIP extra field keeps the stored DEX four-byte aligned.
    let padding = (4 - (out.len() + 30 + name_bytes.len()) % 4) % 4;
    let extra_len = if padding == 0 { 0 } else { padding + 4 };
    out.extend(0x0403_4b50_u32.to_le_bytes());
    for value in [20_u16, 0, 0, 0, 0x21] {
        out.extend(value.to_le_bytes());
    }
    out.extend(crc.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(name_len.to_le_bytes());
    out.extend(u16::try_from(extra_len)?.to_le_bytes());
    out.extend(name_bytes);
    if extra_len != 0 {
        out.extend(0xa1c0_u16.to_le_bytes());
        out.extend(u16::try_from(padding)?.to_le_bytes());
        out.resize(out.len() + padding, 0);
    }
    out.extend(data);
    let new_central = u32::try_from(out.len())?;
    out.extend(&base[central_offset..eocd]);
    out.extend(0x0201_4b50_u32.to_le_bytes());
    for value in [20_u16, 20, 0, 0, 0, 0x21] {
        out.extend(value.to_le_bytes());
    }
    out.extend(crc.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(name_len.to_le_bytes());
    for value in [0_u16, 0, 0, 0] {
        out.extend(value.to_le_bytes());
    }
    out.extend(0_u32.to_le_bytes());
    out.extend(local_offset.to_le_bytes());
    out.extend(name_bytes);
    let new_size = u32::try_from(out.len())? - new_central;
    out.extend(0x0605_4b50_u32.to_le_bytes());
    out.extend([0; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(new_size.to_le_bytes());
    out.extend(new_central.to_le_bytes());
    out.extend([0; 2]);
    Ok(out)
}
fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_match_direct_pipeline_and_are_reproducible() {
        for source in [
            include_str!("../../../testdata/hello.aic"),
            include_str!("../../../testdata/counter.aic"),
            include_str!("../../../testdata/calculator.aic"),
            include_str!("../../../testdata/notes.aic"),
            include_str!("../../../testdata/m5-optimizer.aic"),
            include_str!("../../../testdata/m9-resources.aic"),
            include_str!("../../../testdata/m9-adaptive-lifecycle.aic"),
        ] {
            for optimization_level in [OptimizationLevel::None, OptimizationLevel::Basic] {
                let options = CompilerOptions { optimization_level };
                let result = compile_source(source, options).unwrap();
                let program = aic_opt::optimize(parse_program(source).unwrap(), options);
                assert_eq!(result.dex, aic_dex::encode_activity_dex(&program).unwrap());
                assert_eq!(result, compile_source(source, options).unwrap());
            }
        }
    }
    #[test]
    fn source_locations_survive_api_boundary() {
        let e = compile_source("not a program", CompilerOptions::default()).unwrap_err();
        assert!(e.location.is_some());
        assert!(e.code.starts_with("AIC"));
    }
    #[test]
    fn m9_packages_multiple_verified_activities_deterministically() {
        let source = include_str!("../../../testdata/m9-navigation.aic");
        let result = compile_source(source, CompilerOptions::default()).unwrap();
        assert_eq!(result.ir_version, "0.2");
        assert_eq!(result.activities.len(), 3);
        assert_eq!(
            result
                .dex_files
                .iter()
                .map(|x| x.0.as_str())
                .collect::<Vec<_>>(),
            vec!["classes.dex", "classes2.dex", "classes3.dex"]
        );
        assert!(result.manifest.contains(".MainActivity"));
        assert!(result.manifest.contains(".DetailActivity"));
        assert!(result.manifest.contains(".InputActivity"));
        assert_eq!(
            result,
            compile_source(source, CompilerOptions::default()).unwrap()
        );
    }
    #[test]
    fn m9_exposes_canonical_string_resource_ids() {
        let source = r#"aic_version 0.2 app "Resources" package "dev.aic.resources" {
            resources {
                string greeting locale "zh-TW" = "你好"
                string greeting = "Hello"
                string action_save = "Save"
            }
            activity MainActivity { on_create {
                let text = android.text_view(text: resource.string(greeting) + resource.string(action_save))
                android.set_content_view(text)
            } }
        }"#;
        for optimization_level in [OptimizationLevel::None, OptimizationLevel::Basic] {
            let artifacts = compile_source(source, CompilerOptions { optimization_level }).unwrap();
            assert_eq!(
                artifacts.string_resource_ids,
                vec![
                    ("action_save".to_owned(), 0x7f01_0000),
                    ("greeting".to_owned(), 0x7f01_0001),
                ]
            );
        }
    }
    #[test]
    fn m9_packages_localized_typed_resources_and_assets_deterministically() {
        let source = r##"aic_version 0.2 app "Resources" package "dev.aic.resources" {
            resources {
                string greeting = "Hello"
                string greeting locale "zh-TW" = "你好"
                color primary = "#112233"
                color accent = "#FF445566"
                image launcher project_asset "launcher.png"
                theme app primary primary accent accent
                launcher_icon launcher
            }
            activity MainActivity { on_create {
                let text = android.text_view(text: resource.string(greeting))
                android.set_text_color(view: text, color: resource.color(primary))
                let image = android.image_view(resource: resource.image(launcher))
                android.set_decorative(view: image)
                android.set_content_view(text)
            } }
        }"##;
        let mut png = vec![0_u8; 24];
        png[..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        png[12..16].copy_from_slice(b"IHDR");
        png[19] = 1;
        png[23] = 1;
        let assets = vec![ProjectAsset {
            name: "launcher.png".into(),
            bytes: png,
        }];
        let mut outputs = Vec::new();
        for optimization_level in [OptimizationLevel::None, OptimizationLevel::Basic] {
            let options = CompilerOptions { optimization_level };
            let first = compile_source_with_assets(source, options, &assets).unwrap();
            let second = compile_source_with_assets(source, options, &assets).unwrap();
            assert_eq!(first, second);
            assert!(!first.resources_arsc.is_empty());
            assert_eq!(first.resource_entries[0].0, "res/mipmap/launcher.png");
            assert!(first.manifest.contains("@style/app"));
            assert!(first.manifest.contains("@mipmap/launcher"));
            assert!(first
                .unsigned_apk
                .windows(14)
                .any(|bytes| bytes == b"resources.arsc"));
            outputs.push(first);
        }
        assert_eq!(outputs[0].resources_arsc, outputs[1].resources_arsc);
        assert_eq!(outputs[0].resource_entries, outputs[1].resource_entries);
        assert_eq!(outputs[0].binary_manifest, outputs[1].binary_manifest);
        assert_eq!(outputs[0].dex_files, outputs[1].dex_files);
        assert_eq!(outputs[0].unsigned_apk, outputs[1].unsigned_apk);
        assert_eq!(
            compile_source(source, CompilerOptions::default())
                .unwrap_err()
                .code,
            "AIC8006"
        );
    }
    #[test]
    fn zip_alignment_and_bad_input() {
        let mut empty = vec![0x50, 0x4b, 0x05, 0x06];
        empty.resize(22, 0);
        for name in ["classes.dex", "x", "abcd"] {
            let zip = inject_stored_zip(&empty, name, b"dex payload").unwrap();
            let offset = 30
                + usize::from(u16_at(&zip, 26).unwrap())
                + usize::from(u16_at(&zip, 28).unwrap());
            assert_eq!(offset % 4, 0);
            assert_eq!(&zip[offset..offset + 11], b"dex payload");
            assert!(inject_stored_zip(&zip, name, b"again").is_err());
            for len in 0..zip.len() {
                assert!(inject_stored_zip(&zip[..len], "new", b"").is_err());
            }
        }
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
