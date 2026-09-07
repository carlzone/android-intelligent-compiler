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

## M4 Persistence

M4 adds explicitly governed SharedPreferences and SQLite persistence and a generated offline notes CRUD application. Build and device-test both optimization levels from `compiler/`:

```powershell
./scripts/build-m4.ps1
./scripts/device-smoke-m4.ps1 -SkipBuild
```

See `compiler/docs/m4-persistence.md` for the typed persistence IR, capability policy, and schema limitations.

## M5 Whole-program optimization

M5 adds deterministic call/resource reachability, dead-function and dead-resource removal, complete expression folding for the supported IR, and optimization reports. Build and compare the representative fixture with:

```powershell
./scripts/build-m5.ps1
./scripts/device-smoke-m5.ps1 -SkipBuild
./scripts/benchmark-m5.ps1 -SkipBuild
```

The exact size gate and device methodology are documented in `compiler/docs/m5-whole-program-optimization.md`.

## M6 Android host

The Kotlin host embeds the ARM64 Rust compiler and supports project creation,
AIC source editing, build diagnostics, import/export, approved APK installation,
and launch directly on Android. Prepare and build it from the repository root:

```powershell
./compiler/scripts/build-m6.ps1
```

See [the M6 host guide](compiler/docs/m6-android-host.md) for prerequisites,
device tests, the project archive format, and local signing identity limitations.

## M7 AI integration

M7 adds provider selection for OpenAI, Claude, OpenRouter, Hugging Face,
Ollama, and llama.cpp; a provider-neutral model boundary; strict versioned proposal schema,
three-attempt compiler-feedback repair loop, review-before-apply edits, encrypted
API credentials, and private provenance. Build it with:

```powershell
./compiler/scripts/build-m7.ps1
```

See [the M7 guide](compiler/docs/m7-ai-integration.md). Live-provider acceptance
requires an API key configured locally in the Android host; keys never enter the
project archive, compiler, build artifacts, diagnostics, or provenance log.

## M8 Self-contained APK pipeline

The host now uses Rust binary manifest encoding and aligned APK assembly, retaining
in-process apksig/Android Keystore signing. No bundled AAPT2 or android.jar is needed.

```powershell
./compiler/scripts/build-m8.ps1
```

See [the M8 guide](compiler/docs/m8-toolchain-independence.md) for the supported
subset, independent verification, and per-version device acceptance. Implementation
is available; the full runtime compatibility exit gate remains open.
