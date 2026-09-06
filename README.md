# android-intelligent-compiler
AndroidIntelligentCompiler is an experimental AI-driven Android compiler that transforms high-level app intent into optimized DEX and installable APKs, aiming to minimize dependencies, reduce app size, and enable direct on-device Android app generation without traditional build stacks.

## M0 DEX kernel

The initial Rust workspace lives in `compiler/`. From that directory:

```text
cargo test
cargo clippy --workspace --all-targets
cargo run -p aic-cli -- emit-minimal
```

The last command deterministically writes `compiler/testdata/generated/minimal.dex`. See `compiler/docs/dex-backend.md` for the supported subset and verification options.

## M1 Hello World APK

M1 compiles `compiler/testdata/hello.aic` directly into a framework-Activity DEX and a signed APK without Java/Kotlin source or Gradle. With Android platform 35 and build-tools 35.0.0 installed, run these commands from `compiler/`:

```powershell
./scripts/build-m1.ps1
./scripts/verify-m1.ps1
./scripts/device-smoke-m1.ps1
```

The final command requires exactly one authorized physical ARM64 device. See `compiler/docs/m1-hello-apk.md` for the supported IR subset, artifacts, signing policy, and exit-gate evidence.

## M3 Interactive UI

M3 adds activity state, buttons, integer inputs, scroll/layout primitives, click handlers, and runtime UI updates. Counter and calculator fixtures can be built and tested from `compiler/`:

```powershell
./scripts/build-m3.ps1
./scripts/device-smoke-m3.ps1 -SkipBuild
```

See `compiler/docs/m3-interactive-ui.md` for the IR contract and recreation policy.
