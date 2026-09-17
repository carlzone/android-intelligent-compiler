//! Thin JNI adapter. No compiler logic or signing secrets cross this boundary.
use aic_opt::{CompilerOptions, OptimizationLevel};
use jni::{
    objects::{JClass, JString},
    sys::{jint, jstring},
    JNIEnv,
};
use serde_json::json;
use std::{fs, path::Path};

/// Shared with desktop tests; the output directory is private host-managed storage.
#[must_use]
pub fn compile_request(source: &str, assets: &Path, output: &Path, level: i32) -> String {
    if !matches!(level, 0 | 1) {
        return failure("validate", "AIC6001", "Optimization level must be 0 or 1");
    }
    let options = CompilerOptions {
        optimization_level: if level == 0 {
            OptimizationLevel::None
        } else {
            OptimizationLevel::Basic
        },
    };
    let assets = match read_assets(assets) {
        Ok(assets) => assets,
        Err(e) => return failure("validate", "AIC6005", &e.to_string()),
    };
    match aic_build::compile_source_with_assets(source, options, &assets) {
        Ok(a) => {
            let write = || -> std::io::Result<()> {
                fs::create_dir_all(output)?;
                fs::write(output.join("AndroidManifest.axml"), &a.binary_manifest)?;
                fs::write(output.join("unsigned.apk"), &a.unsigned_apk)?;
                fs::write(output.join("classes.dex"), &a.dex)?;
                for (name, bytes) in &a.dex_files {
                    fs::write(output.join(name), bytes)?;
                }
                fs::write(output.join("AndroidManifest.xml"), &a.manifest)?;
                fs::write(output.join("build-profile.txt"), &a.build_profile)?;
                if !a.resources_arsc.is_empty() {
                    fs::write(output.join("resources.arsc"), &a.resources_arsc)?;
                }
                for (name, bytes) in &a.resource_entries {
                    let path = output.join(name);
                    let parent = path.parent().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "resource entry has no parent",
                        )
                    })?;
                    fs::create_dir_all(parent)?;
                    fs::write(path, bytes)?;
                }
                Ok(())
            };
            match write() {
                Ok(()) => json!({"ok":true,"package":a.package,"activity":a.activity,"activities":a.activities,"irVersion":a.ir_version,"catalogVersion":a.catalog_version,"migrationRequired":a.ir_version=="0.1","report":a.build_profile,"diagnostics":[]}).to_string(),
                Err(e) => failure("write", "AIC6002", &e.to_string()),
            }
        }
        Err(e) => json!({"ok":false,"diagnostics":[{"stage":e.stage,"code":e.code,"message":e.message,
            "location":e.location.map(|s| json!({"line":s.start.line,"column":s.start.column,"endLine":s.end.line,"endColumn":s.end.column}))}]}).to_string(),
    }
}

/// Validates and lowers a proposal without writing an artifact. This is the AI
/// trust boundary: success means the same deterministic compiler accepted it.
#[must_use]
pub fn validate_request(source: &str, assets: &Path, level: i32) -> String {
    if !matches!(level, 0 | 1) {
        return failure("validate", "AIC6001", "Optimization level must be 0 or 1");
    }
    let options = CompilerOptions {
        optimization_level: if level == 0 {
            OptimizationLevel::None
        } else {
            OptimizationLevel::Basic
        },
    };
    let assets = match read_assets(assets) {
        Ok(assets) => assets,
        Err(e) => return failure("validate", "AIC6005", &e.to_string()),
    };
    match aic_build::compile_source_with_assets(source, options, &assets) {
        Ok(a) => json!({"ok":true,"package":a.package,"activity":a.activity,"activities":a.activities,"irVersion":a.ir_version,"catalogVersion":a.catalog_version,"migrationRequired":a.ir_version=="0.1","report":a.build_profile,"diagnostics":[]}).to_string(),
        Err(e) => json!({"ok":false,"diagnostics":[{"stage":e.stage,"code":e.code,"message":e.message,
            "location":e.location.map(|s| json!({"line":s.start.line,"column":s.start.column,"endLine":s.end.line,"endColumn":s.end.column}))}]}).to_string(),
    }
}

fn read_assets(directory: &Path) -> std::io::Result<Vec<aic_build::ProjectAsset>> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    entries
        .into_iter()
        .map(|entry| {
            if !entry.file_type()?.is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "asset directory may contain regular files only",
                ));
            }
            let name = entry.file_name().into_string().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "asset filename must be UTF-8",
                )
            })?;
            Ok(aic_build::ProjectAsset {
                name,
                bytes: fs::read(entry.path())?,
            })
        })
        .collect()
}

fn failure(stage: &str, code: &str, message: &str) -> String {
    json!({"ok":false,"diagnostics":[{"stage":stage,"code":code,"message":message,"location":null}]}).to_string()
}

// JNI receives JVM-owned references valid for this invocation. jni handles all
// string conversions (including UTF-16); returned strings transfer a local ref
// to the JVM. Unwinding is caught before it can cross the C ABI.
#[no_mangle]
pub extern "system" fn Java_dev_aic_host_NativeCompiler_compile(
    mut env: JNIEnv,
    _: JClass,
    source: JString,
    assets: JString,
    output: JString,
    level: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let source: String = env.get_string(&source)?.into();
        let assets: String = env.get_string(&assets)?.into();
        let output: String = env.get_string(&output)?.into();
        Ok::<_, jni::errors::Error>(compile_request(
            &source,
            Path::new(&assets),
            Path::new(&output),
            level,
        ))
    }));
    let response = match result {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => failure("bridge", "AIC6003", &e.to_string()),
        Err(_) => failure("bridge", "AIC6004", "Internal compiler failure"),
    };
    env.new_string(response)
        .map_or(std::ptr::null_mut(), jni::objects::JString::into_raw)
}

#[no_mangle]
pub extern "system" fn Java_dev_aic_host_NativeCompiler_validate(
    mut env: JNIEnv,
    _: JClass,
    source: JString,
    assets: JString,
    level: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let source: String = env.get_string(&source)?.into();
        let assets: String = env.get_string(&assets)?.into();
        Ok::<_, jni::errors::Error>(validate_request(&source, Path::new(&assets), level))
    }));
    let response = match result {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => failure("bridge", "AIC6003", &e.to_string()),
        Err(_) => failure("bridge", "AIC6004", "Internal compiler failure"),
    };
    env.new_string(response)
        .map_or(std::ptr::null_mut(), jni::objects::JString::into_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapter_writes_identical_artifacts_and_preserves_errors() {
        let dir = std::env::temp_dir().join(format!("aic-jni-test-{}", std::process::id()));
        let assets = dir.join("assets");
        fs::create_dir_all(&assets).unwrap();
        for level in [0, 1] {
            let source = include_str!("../../../testdata/notes.aic");
            let result: serde_json::Value =
                serde_json::from_str(&compile_request(source, &assets, &dir, level)).unwrap();
            assert_eq!(result["ok"], true);
            let direct = aic_build::compile_source(
                source,
                CompilerOptions {
                    optimization_level: if level == 0 {
                        OptimizationLevel::None
                    } else {
                        OptimizationLevel::Basic
                    },
                },
            )
            .unwrap();
            assert_eq!(fs::read(dir.join("classes.dex")).unwrap(), direct.dex);
            assert_eq!(
                fs::read(dir.join("AndroidManifest.axml")).unwrap(),
                direct.binary_manifest
            );
            assert_eq!(
                fs::read(dir.join("unsigned.apk")).unwrap(),
                direct.unsigned_apk
            );
            assert_eq!(
                fs::read_to_string(dir.join("AndroidManifest.xml")).unwrap(),
                direct.manifest
            );
        }
        let invalid: serde_json::Value =
            serde_json::from_str(&compile_request("bad input", &assets, &dir, 1)).unwrap();
        assert_eq!(invalid["ok"], false);
        assert!(invalid["diagnostics"][0]["location"].is_object());
        assert!(compile_request("", &assets, &dir, 2).contains("AIC6001"));
        assert!(compile_request(
            include_str!("../../../testdata/hello.aic"),
            &assets,
            &dir.join("classes.dex"),
            1
        )
        .contains("AIC6002"));
        let valid: serde_json::Value = serde_json::from_str(&validate_request(
            include_str!("../../../testdata/counter.aic"),
            &assets,
            1,
        ))
        .unwrap();
        assert_eq!(valid["ok"], true);
        let resource_source = r#"aic_version 0.2 app "Asset" package "dev.aic.asset" {
            resources { image logo project_asset "logo.png" }
            activity MainActivity { on_create {
                let logo = android.image_view(resource: resource.image(logo))
                android.set_decorative(view: logo)
                android.set_content_view(logo)
            } }
        }"#;
        let mut png = vec![0_u8; 24];
        png[..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        png[12..16].copy_from_slice(b"IHDR");
        png[19] = 1;
        png[23] = 1;
        fs::write(assets.join("logo.png"), png).unwrap();
        let resource_result: serde_json::Value = serde_json::from_str(&compile_request(
            resource_source,
            &assets,
            &dir.join("resource-output"),
            1,
        ))
        .unwrap();
        assert_eq!(resource_result["ok"], true);
        assert!(dir.join("resource-output/resources.arsc").is_file());
        assert!(dir.join("resource-output/res/drawable/logo.png").is_file());
        assert!(!dir.join("validation-only").exists());
        let invalid: serde_json::Value = serde_json::from_str(&validate_request(
            "aic_version 0.1\napp \"x\" package \"dev.aic.x\" { capability camera }",
            &assets,
            1,
        ))
        .unwrap();
        assert_eq!(invalid["ok"], false);
        fs::remove_dir_all(dir).unwrap();
    }
}
