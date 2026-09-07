# M8 acceptance evidence

Status: implementation complete; compatibility exit gate open.

Date: 2026-09-08  
Profile: `android-35` (DEX 035, generated minSdk 23, targetSdk 35)  
Host: ARM64, minSdk 30

## Implemented pipeline

The supported generated-app path is now:

```text
AIC IR -> verified/optimized DEX -> compiler-owned manifest model
       -> binary Android XML -> deterministic aligned APK
       -> in-process AOSP apksig + Android Keystore -> PackageInstaller
```

The host no longer ships or executes AAPT2 and no longer ships `android.jar`.
Host APK inspection confirms the absence of `libaapt2.so`, `android.jar`, and
the former bootstrap asset directory. Kotlin source inspection confirms that the
generated-app build path has no `ProcessBuilder` or runtime-exec call.

## Passing automated evidence

- `cargo fmt --manifest-path compiler/Cargo.toml --all --check`
- `cargo test --manifest-path compiler/Cargo.toml --workspace --locked`
- `cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets --locked -- -D warnings`
- Clean host build followed by `./compiler/scripts/build-m8.ps1 -Offline`
- Host JVM tests: 8 tests, 0 failures; Android lint: 0 errors
- Desktop M8 corpus: Hello, Counter, Calculator, Notes, Optimizer, Unicode-label,
  and 32,768-UTF-16-unit label fixtures, each at O0 and O1 (14 cases)
- Every desktop case is byte-for-byte reproducible and passes independent AAPT2
  manifest decoding/semantic comparison, DEX inspection, ZIP CRC/order/fixed-time
  and four-byte-alignment checks
- Every desktop signed APK passes official `apksigner verify` for API 23-36 with
  v1/v2 enabled; a modified DEX byte is rejected in every case
- Android API 35 x86_64 emulator runtime: all five generated-app fixtures pass at
  O0 and O1, including launch, counter/optimizer state changes, calculator
  arithmetic, and Notes create/read/update/delete plus process-death relaunch
- Android API 36 physical ARM64 host, offline: instrumentation, all ten
  Android-Keystore-signed artifact parity checks, PackageInstaller cancellation
  and recovery, counter install/update, calculator, Notes persistence, and the
  host-built O0/O1 runtime corpus passed. The final Optimizer O0/O1 interactions
  were manually confirmed by the user on 2026-09-08 after Android/HyperOS
  cancelled the automated ADB install.

Generated evidence is written below `compiler/testdata/generated/m8/` and is
ignored by Git. `desktop-verification.json`, per-device `compatibility.json`, and
`compatibility-matrix.json` include or are bound to the current implementation
fingerprint so stale results cannot satisfy a changed build.

## Open compatibility gate

The milestone must not be marked complete yet. Current runtime matrix status:

| API | Generated app | Native ARM64 host, offline |
| --- | --- | --- |
| 23-29 | Missing | Not applicable |
| 30-34 | Missing | Missing |
| 35 | Pass | Missing |
| 36 | Pass | Pass |

The API 35 generated-app evidence came from a Pixel Tablet x86_64 Android 15
emulator. Its ABI list also advertises ARM64 translation, but translated execution
does not satisfy the native ARM64 host row. The API 36 host evidence came from a
physical ARM64 Redmi 2312DRA50G running Android 16 while offline.
The API 36 host test compiled and signed the generated applications in AIC Host,
then installed, launched, and behavior-tested the complete O0/O1 corpus. That
same evidence satisfies the API 36 generated-app runtime row; the separate
desktop-signed `--generated` route is an additional packaging path, not a second
requirement for runtime compatibility.

Run `python host/scripts/device-m8.py` for each required device and then
`python compiler/scripts/report-m8-matrix.py --require-complete`. The latter exits
nonzero until generated-app APIs 23-36 and native ARM64 host APIs 30-36 all pass.

## Scope and limitations

The supported resource profile remains intentionally resource-free: programmatic
Views, literal labels, one launcher Activity, platform theme reference, and no
generated-app permissions for the current capabilities. `resources.arsc`, custom
resource references, icons, layouts, localization, and new permission-bearing
features remain M9 work. Signing retains in-process AOSP apksig 8.10.1 and the
existing Android Keystore debug identity; v3/v4 and release distribution remain
outside M8.

See [ADR 0006](adr/0006-m8-toolchain-independence.md) and the
[M8 guide](m8-toolchain-independence.md) for the contracts and reproduction steps.
