use aic_build::{compile_source_with_assets, inject_stored_zip, ProjectAsset};
use aic_dex::encode_minimal_dex;
use aic_ir::{migrate_source, MinimalClass};
use aic_opt::{CompilerOptions, OptimizationLevel};
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
        Some("migrate") => migrate(&args[1..]),
        Some("assemble-apk") => assemble(&args[1..]),
        _ => Err("usage: aic-cli emit-minimal | compile --input <file> --output-dir <dir> --profile android-35 [--opt-level 0|1] [--assets-dir <dir>] | migrate --input <file> --output <file> --to 0.2 | assemble-apk --base <apk> --dex <dex> --output <apk>".into()),
    }
}
fn migrate(args: &[OsString]) -> Result<(), Box<dyn Error>> {
    let input = option(args, "--input")?;
    let output = option(args, "--output")?;
    if option(args, "--to")? != Path::new("0.2") {
        return Err("unsupported migration target; expected 0.2".into());
    }
    let migrated = migrate_source(&fs::read_to_string(input)?)?;
    write_file(&output, migrated.as_bytes())?;
    println!("migrated {}", output.display());
    Ok(())
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
    let assets = optional(args, "--assets-dir")
        .map(|directory| read_assets(&directory))
        .transpose()?
        .unwrap_or_default();
    let artifacts = compile_source_with_assets(
        &fs::read_to_string(input)?,
        CompilerOptions { optimization_level },
        &assets,
    )?;
    fs::create_dir_all(&output)?;
    write_file(
        &output.join("AndroidManifest.axml"),
        &artifacts.binary_manifest,
    )?;
    if !artifacts.resources_arsc.is_empty() {
        write_file(&output.join("resources.arsc"), &artifacts.resources_arsc)?;
    }
    for (name, bytes) in &artifacts.resource_entries {
        write_file(&output.join(name), bytes)?;
    }
    write_file(&output.join("unsigned.apk"), &artifacts.unsigned_apk)?;
    write_file(&output.join("classes.dex"), &artifacts.dex)?;
    for (name, bytes) in &artifacts.dex_files {
        write_file(&output.join(name), bytes)?;
    }
    write_file(
        &output.join("AndroidManifest.xml"),
        artifacts.manifest.as_bytes(),
    )?;
    write_file(
        &output.join("build-profile.txt"),
        artifacts.build_profile.as_bytes(),
    )?;
    println!("compiled {}", output.display());
    Ok(())
}
fn read_assets(directory: &Path) -> Result<Vec<ProjectAsset>, Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    entries
        .into_iter()
        .map(|entry| {
            if !entry.file_type()?.is_file() {
                return Err("assets directory may contain files only".into());
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "asset filename must be UTF-8")?;
            Ok(ProjectAsset {
                name,
                bytes: fs::read(entry.path())?,
            })
        })
        .collect()
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
