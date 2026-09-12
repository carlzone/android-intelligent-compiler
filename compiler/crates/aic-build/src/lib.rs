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
}

/// Compile validated AIC source without filesystem or Android host dependencies.
/// # Errors
/// Returns a source-located diagnostic or a typed DEX lowering failure.
pub fn compile_source(
    source: &str,
    options: CompilerOptions,
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
    let manifest = aic_res::Manifest::new_multi(&program.package, &program.label, &activity_names);
    let binary_manifest = manifest.binary().map_err(|e| BuildError {
        stage: "package",
        code: "AIC8001",
        message: e.to_string(),
        location: None,
    })?;
    let unsigned_apk =
        assemble_apk_files(&binary_manifest, &dex_files).map_err(|e| BuildError {
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
    })
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
