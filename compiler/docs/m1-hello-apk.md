# M1 Hello World APK

The M1 fixture is `testdata/hello.aic`. It supports exactly one app, one Activity, one `on_create` block, a vertical `LinearLayout`, a `TextView`, `add_view`, and `set_content_view`. Diagnostics have stable `AIC` codes and line/column locations where applicable. Anything outside this vocabulary is rejected before DEX generation.

## Prerequisites

- Rust stable with rustfmt and clippy
- `ANDROID_SDK_ROOT`
- Android platform 35 and build-tools 35.0.0
- A JDK providing keytool
- For the exit gate, exactly one authorized physical ARM64 device

From `compiler/`, run:

```powershell
./scripts/build-m1.ps1
./scripts/verify-m1.ps1
./scripts/device-smoke-m1.ps1
```

The build produces ignored artifacts under `testdata/generated/m1`: the text manifest, profile record, DEX, intermediate APKs, signed APK, tool audit, and verification report. The audit is checked for javac, kotlinc, Gradle, and Gradle-wrapper invocations. A default debug key is generated once under ignored `.aic/`; pass `-Keystore` to use another key.

The device script fails on zero or multiple matching devices. It installs and launches `dev.aic.generated.hello/.MainActivity`, verifies the exact visible text through UI Automator, checks the crash buffer, and writes `device-evidence.txt`. Successful desktop verification alone does not satisfy the M1 physical-device exit gate.
