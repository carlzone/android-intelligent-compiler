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
pub fn compile_request(source: &str, output: &Path, level: i32) -> String {
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
    match aic_build::compile_source(source, options) {
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
pub fn validate_request(source: &str, level: i32) -> String {
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
    match aic_build::compile_source(source, options) {
        Ok(a) => json!({"ok":true,"package":a.package,"activity":a.activity,"activities":a.activities,"irVersion":a.ir_version,"catalogVersion":a.catalog_version,"migrationRequired":a.ir_version=="0.1","report":a.build_profile,"diagnostics":[]}).to_string(),
        Err(e) => json!({"ok":false,"diagnostics":[{"stage":e.stage,"code":e.code,"message":e.message,
            "location":e.location.map(|s| json!({"line":s.start.line,"column":s.start.column,"endLine":s.end.line,"endColumn":s.end.column}))}]}).to_string(),
    }
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
    output: JString,
    level: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let source: String = env.get_string(&source)?.into();
        let output: String = env.get_string(&output)?.into();
        Ok::<_, jni::errors::Error>(compile_request(&source, Path::new(&output), level))
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
    level: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let source: String = env.get_string(&source)?.into();
        Ok::<_, jni::errors::Error>(validate_request(&source, level))
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
        for level in [0, 1] {
            let source = include_str!("../../../testdata/notes.aic");
            let result: serde_json::Value =
                serde_json::from_str(&compile_request(source, &dir, level)).unwrap();
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
            serde_json::from_str(&compile_request("bad input", &dir, 1)).unwrap();
        assert_eq!(invalid["ok"], false);
        assert!(invalid["diagnostics"][0]["location"].is_object());
        assert!(compile_request("", &dir, 2).contains("AIC6001"));
        assert!(compile_request(
            include_str!("../../../testdata/hello.aic"),
            &dir.join("classes.dex"),
            1
        )
        .contains("AIC6002"));
        let valid: serde_json::Value = serde_json::from_str(&validate_request(
            include_str!("../../../testdata/counter.aic"),
            1,
        ))
        .unwrap();
        assert_eq!(valid["ok"], true);
        assert!(!dir.join("validation-only").exists());
        let invalid: serde_json::Value = serde_json::from_str(&validate_request(
            "aic_version 0.1\napp \"x\" package \"dev.aic.x\" { capability camera }",
            1,
        ))
        .unwrap();
        assert_eq!(invalid["ok"], false);
        fs::remove_dir_all(dir).unwrap();
    }
}
