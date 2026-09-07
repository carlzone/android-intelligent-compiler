# M8 toolchain independence

M8 removes AAPT2 and android.jar from the supported on-device generated-app path.
The Rust compiler now emits deterministic binary manifests and unsigned aligned
APKs. The host signs in-process with the existing AOSP apksig/Android Keystore
identity and uses the existing approved PackageInstaller workflow.

## Build and independent verification

From the repository root, with the existing Rust/Android SDK/NDK/JDK prerequisites:

```powershell
./compiler/scripts/build-m8.ps1
# With host-development dependencies already cached:
./compiler/scripts/build-m8.ps1 -Offline
cargo fmt --manifest-path compiler/Cargo.toml --all --check
cargo test --manifest-path compiler/Cargo.toml --workspace --locked
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets --locked -- -D warnings
```

No prepare-m6.ps1 step is needed. The older M6/M7 build entry points now build the
current self-contained host. Their PrepareBootstrap switch is retained as a
warning-only compatibility option. Historical bootstrap preparation and desktop
M1-M5 scripts remain available as reference workflows; they are not dependencies
of the M8 host or its generated-app builds. Gradle excludes stale bootstrap assets
and libaapt2.so, and the About screen uses shipped toolchain notices instead.

`verify-m8.py` runs the five application fixtures plus Unicode and long-label
cases at O0/O1. It compares independent AAPT2 manifest dumps, ignoring only the
four AAPT2-added compile/platform metadata attributes and XML line numbers. It
checks exact repeatability, ZIP CRCs/order/alignment, DEX inspection, Android-35
framework IDs, v1/v2 signatures for API 23-36, and tamper rejection. Desktop
signing is explicitly an oracle path, not evidence of Android Keystore execution.
Pass --host-apk to additionally inspect the shipped host and source boundary.

## Compiler interface and subset

`aic-build::BuildArtifacts` adds binary_manifest and unsigned_apk. CLI `compile`
and JNI compile persist AndroidManifest.axml and unsigned.apk, retaining
classes.dex, textual AndroidManifest.xml and build-profile.txt. NativeCompiler's
separate assemble method is retired. No project/archive/schema migration occurs.

The model supports package, literal application label, framework theme reference,
SDK levels, Activity name/exported state, MAIN action and LAUNCHER category.
The existing persistence capabilities need no Android permissions. The verifier
still rejects unsupported or undeclared capability use; the writer accepts no
arbitrary XML. App resources.arsc, custom resource references, icons, resource
layouts, localization and new permissions are outside this profile.

Unsigned artifacts are byte-for-byte reproducible with fixed ZIP metadata.
Signed bytes retain the existing identity/metadata exception. Rebuilding with the
same host installation preserves the signing alias and update compatibility;
importing projects does not transfer signing keys. Keys never enter Rust or AI
interfaces. AIC8001 identifies manifest encoding errors, AIC8002 APK assembly
errors, and AIC8003 host signing failures. No partial signed output is installable.

## Runtime compatibility acceptance

Use test devices/emulators with the required API versions. Run desktop acceptance
first to create the local test key and CLI. Select each device explicitly:

```powershell
python host/scripts/device-m8.py --serial SERIAL --generated
# ARM64 API 30-36; enable airplane mode and disable Wi-Fi first:
python host/scripts/device-m8.py --serial SERIAL --host --require-offline
python compiler/scripts/report-m8-matrix.py
python compiler/scripts/report-m8-matrix.py --require-complete
```

The host runner installs the host/instrumentation APKs, compiles and signs all five
fixtures at O0/O1 in the host UID, compares JNI and desktop artifacts, independently
verifies pulled signed APKs, and runs approved installer cancellation/update,
interaction and CRUD/relaunch scenarios. Normal Android install-source approval
must already be enabled; device security dialogs may require attention. Host
fixtures retain the reserved M6 acceptance package names and project labels for
compatibility with the existing driver. An emulator with ARM64 native translation
can supply supplemental host evidence, but never satisfies a native ARM64 matrix row.
Generated-only tests use dev.aic.m8.* and
ADB installation, and can run on non-ARM64 devices because their APKs are DEX-only.
Use dedicated test devices: acceptance creates projects/apps and modifies test data.

Per-device profiles, logs and results go under ignored
compiler/testdata/generated/m8/devices. The matrix accepts only actual passing
runtime results; host rows additionally require recorded offline ARM64 execution.
Every API 23-36 needs a generated-app pass and every API 30-36 needs a host pass.
Desktop checks, missing devices and partial/failed runs never satisfy those rows.
The matrix rejects rows with a different compiler/host/test input fingerprint.
Keep each evidence run with its matching source revision and host APK; rerun the
matrix after compiler or host changes rather than carrying older acceptance forward.

See [ADR 0006](adr/0006-m8-toolchain-independence.md) and
[acceptance evidence](m8-acceptance-evidence.md). M8 remains open until its complete
runtime matrix passes.
