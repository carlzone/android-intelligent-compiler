# M6 acceptance evidence

Status: **Pass (2026-09-07)**

## Automated gates

- Rust formatting, workspace tests, and Clippy with warnings denied pass. The
  new `aic-build` tests cover all four fixtures at O0/O1, deterministic output,
  source-located errors, checked ZIP parsing and four-byte DEX alignment. The
  `aic-jni` test proves byte parity with the reusable compiler API.
- The ARM64 JNI library cross-compiles with NDK 27.1.12297006. The Kotlin host,
  Android test APK, JVM archive tests and Android lint build successfully with
  Gradle 8.14.3, AGP/apksig 8.10.1 and Kotlin 2.1.20.
- Archive tests cover Unicode round trips, exact canonical fields, unsupported
  settings, invalid UTF-8, unknown/traversal/duplicate entries and size limits.
- AAPT2 was reproduced from the pinned v1.1.0 source revision and Android 35
  platform resources. The source-built ARM64 executable ran inside the host
  sandbox; no executable was copied to or launched from writable app storage.

## Device and independent verification

The authorized device was `2312DRA50G`, ABI `arm64-v8a`, Android 16. All test
inputs and generated-app builds ran without network access from the host.

- The host built Hello World, Counter, Calculator and Notes at O0 and O1 through
  JNI, AAPT2, aligned APK assembly and Android-Keystore-backed AOSP apksig.
- Every device-built DEX and manifest matched the desktop CLI byte-for-byte.
  Desktop `dexdump`, `zipalign -c -v 4`, and `apksigner verify --verbose`
  independently passed all eight outputs. No manifest requested permissions.
- The host used Android's unknown-source settings and `PackageInstaller`
  confirmation. Installation cancellation was reported without altering the
  project. An approved install launched Hello World. Same-key reinstall passed.
- Counter increment/reset and calculator add/multiply passed after host-driven
  installation and launch. Notes create/read/update/delete passed, and the
  record survived force-stop/relaunch. The run added no host/generated-app crash
  records.
- Installing a differently signed build over `dev.aic.m6.counter` produced
  `INSTALL_FAILED_UPDATE_INCOMPATIBLE`. The host receiver test preserved the
  corresponding signing-conflict diagnostic and cleared its stored session.
  The host-signed reference build was restored afterward.
- The instrumentation suite passed Unicode editor storage, Activity recreation
  during a build, detection of a process-death build checkpoint, malformed
  source locations and output-storage failures.
- A project was created through the host UI, exported through Android's document
  picker, and imported as a distinct private project. Source, display name,
  profile and optimization level were preserved; local IDs and signing material
  were not exported.

Generated raw evidence is intentionally ignored under
`compiler/testdata/generated/m6/`; reproduce it with the commands in
`compiler/docs/m6-android-host.md`.

## Exit decision

The full project create/import/edit, compile, package, user-approved install and
launch workflow succeeds on the reference device without a desktop computer at
generated-app build time. Compiler warnings/errors are actionable, project
archives preserve canonical data, and the compiler remains independently
testable. M6 passes. Toolchain independence and compatibility beyond the single
declared ARM64/API profile remain M8 work.
