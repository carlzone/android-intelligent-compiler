use aic_dex::{encode_activity_dex, encode_minimal_dex};
use aic_ir::{parse_program, MinimalClass, Program};
use aic_opt::{optimize, CompilerOptions, OptimizationLevel};
use std::{
    env,
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.first().and_then(|v| v.to_str()) {
        Some("emit-minimal") if args.len() == 1 => emit_minimal(),
        Some("compile") => compile(&args[1..]),
        Some("assemble-apk") => assemble(&args[1..]),
        _ => Err("usage: aic-cli emit-minimal | compile --input <file> --output-dir <dir> --profile android-35 [--opt-level 0|1] | assemble-apk --base <apk> --dex <dex> --output <apk>".into()),
    }
}
fn emit_minimal() -> Result<(), Box<dyn Error>> {
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/generated/minimal.dex");
    write_file(
        &output,
        &encode_minimal_dex(&MinimalClass::new("Ldev/aic/generated/Minimal;")?)?,
    )?;
    println!("wrote {}", output.display());
    Ok(())
}
fn compile(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    let input = option(args, "--input")?;
    let output = option(args, "--output-dir")?;
    let profile = option(args, "--profile")?;
    if profile != Path::new("android-35") {
        return Err("unsupported profile; expected android-35".into());
    }
    let level = optional(args, "--opt-level").unwrap_or_else(|| PathBuf::from("1"));
    let optimization_level = match level.to_str() {
        Some("0") => OptimizationLevel::None,
        Some("1") => OptimizationLevel::Basic,
        _ => return Err("unsupported optimization level; expected 0 or 1".into()),
    };
    let program = optimize(
        parse_program(&fs::read_to_string(input)?)?,
        CompilerOptions { optimization_level },
    );
    fs::create_dir_all(&output)?;
    write_file(&output.join("classes.dex"), &encode_activity_dex(&program)?)?;
    write_file(
        &output.join("AndroidManifest.xml"),
        manifest(&program).as_bytes(),
    )?;
    write_file(
        &output.join("build-profile.txt"),
        format!(
            "profile=android-35\ndex=035\nminSdk=23\ntargetSdk=35\noptLevel={}\n",
            i32::from(optimization_level != OptimizationLevel::None)
        )
        .as_bytes(),
    )?;
    println!("compiled {}", output.display());
    Ok(())
}
fn assemble(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    let base = fs::read(option(args, "--base")?)?;
    let dex = fs::read(option(args, "--dex")?)?;
    let output = option(args, "--output")?;
    write_file(&output, &inject_stored_zip(&base, "classes.dex", &dex)?)?;
    println!("assembled {}", output.display());
    Ok(())
}
fn option(args: &[OsString], name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let at = args
        .iter()
        .position(|v| v == name)
        .ok_or_else(|| format!("missing {name}"))?;
    args.get(at + 1)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value for {name}").into())
}
fn optional(args: &[OsString], name: &str) -> Option<PathBuf> {
    args.iter()
        .position(|v| v == name)
        .and_then(|at| args.get(at + 1))
        .map(PathBuf::from)
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn manifest(p: &Program) -> String {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\" package=\"{}\">\n  <uses-sdk android:minSdkVersion=\"23\" android:targetSdkVersion=\"35\" />\n  <application android:label=\"{}\" android:theme=\"@android:style/Theme.Material.Light.NoActionBar\">\n    <activity android:name=\".{}\" android:exported=\"true\">\n      <intent-filter>\n        <action android:name=\"android.intent.action.MAIN\" />\n        <category android:name=\"android.intent.category.LAUNCHER\" />\n      </intent-filter>\n    </activity>\n  </application>\n</manifest>\n", xml(&p.package), xml(&p.label), xml(&p.activity.name))
}

fn inject_stored_zip(base: &[u8], name: &str, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let eocd = base
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x05, 0x06])
        .ok_or("base APK has no ZIP end record")?;
    if eocd + 22 > base.len() {
        return Err("truncated ZIP end record".into());
    }
    let entries = u16::from_le_bytes(base[eocd + 10..eocd + 12].try_into()?);
    let central_size = u32::from_le_bytes(base[eocd + 12..eocd + 16].try_into()?) as usize;
    let central_offset = u32::from_le_bytes(base[eocd + 16..eocd + 20].try_into()?) as usize;
    if central_offset + central_size > base.len() {
        return Err("invalid ZIP central directory".into());
    }
    let mut out = base[..central_offset].to_vec();
    let local_offset = u32::try_from(out.len())?;
    let crc = crc32(data);
    let size = u32::try_from(data.len())?;
    let name_bytes = name.as_bytes();
    let name_len = u16::try_from(name_bytes.len())?;
    out.extend(0x0403_4b50_u32.to_le_bytes());
    out.extend(20_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(crc.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(name_len.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(name_bytes);
    out.extend(data);
    let new_central = u32::try_from(out.len())?;
    out.extend(&base[central_offset..central_offset + central_size]);
    out.extend(0x0201_4b50_u32.to_le_bytes());
    out.extend(20_u16.to_le_bytes());
    out.extend(20_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(crc.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(name_len.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u32.to_le_bytes());
    out.extend(local_offset.to_le_bytes());
    out.extend(name_bytes);
    let new_size = u32::try_from(out.len())? - new_central;
    out.extend(0x0605_4b50_u32.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
    out.extend((entries + 1).to_le_bytes());
    out.extend((entries + 1).to_le_bytes());
    out.extend(new_size.to_le_bytes());
    out.extend(new_central.to_le_bytes());
    out.extend(0_u16.to_le_bytes());
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
    fn escapes_manifest() {
        assert_eq!(xml("a&b"), "a&amp;b");
    }
    #[test]
    fn known_crc() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
