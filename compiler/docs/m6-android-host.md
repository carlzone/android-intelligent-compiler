# M6 Android host

> Historical M6 baseline. The current build uses the self-contained M8 pipeline;
> bootstrap provisioning below is no longer required. See [M8](m8-toolchain-independence.md).

The Kotlin host provides project creation, `.aic` editing, O0/O1 compilation,
diagnostics, portable import/export, platform-approved installation, and launch.
It supports ARM64 Android 11/API 30 and newer; the acceptance device is the
existing Android 16 `2312DRA50G`. Other Android versions remain untested until M8.

## Build and provision

Prerequisites: JDK 17+ (`JAVA_HOME`), Python 3, Git, Rust with the
`aarch64-linux-android` target, Android platform 35, NDK 27.1.12297006, and SDK
CMake 3.22.1. Set `ANDROID_SDK_ROOT`. Run from the repository root:

```powershell
rustup target add aarch64-linux-android
./compiler/scripts/prepare-m6.ps1
./compiler/scripts/build-m6.ps1
adb install -r host/app/build/outputs/apk/debug/app-debug.apk
```

The preparation script fetches pinned public source and protoc 21.12, verifies
the protoc and platform SHA-256 values, applies documented upstream/Windows
adaptations, and builds static ARM64 AAPT2. It writes generated bootstrap assets
and notices under ignored host directories. Subsequent host builds use those
assets. Gradle's wrapper distribution is pinned by SHA-256. Toolchain versions
and the resulting AAPT2 hash are bundled in `bootstrap/versions.txt`.

The host and generated-app keys are separate. The desktop host debug key lives
under ignored `compiler/.aic/host-debug.keystore`; never commit it. The
generated-app key is created on the Android device through Android Keystore.
The host has no `INTERNET` permission and downloads no executable tools.

## Use

1. Tap **New**, select Hello World, Counter, Calculator, or Notes, and name it.
   Each new project gets a distinct generated package name. **Projects** switches
   between saved projects. **Import** accepts existing `.aic` files or archives.
2. Edit AIC source, choose O0/O1, and tap **Build**. Edits autosave after a short
   pause and on leaving the screen; **Save** also saves explicitly. Build uses
   one saved immutable snapshot, runs off the UI thread, and displays diagnostic
   codes with line/column locations. The source language is the existing AIC IR,
   not Kotlin, Java, or natural language.
3. Tap **Install**. On first use, enable installation from AIC Host in Android
   settings, return, and tap Install again. Android presents its own approval
   dialog. Device-specific security/scan dialogs can require additional action.
   Denying installation does not alter the project.
4. Tap **Launch** after installation succeeds. The generated app opens in its
   own task. The host shows installer/launch errors but cannot read unrestricted
   generated-app runtime logs. ADB is used only by acceptance tooling for those
   diagnostics.
5. **Export** writes a project archive through Android's document picker.
   Importing creates a new local project; it never replaces an existing project.

Installation is disabled when the current source/settings differ from the
successful build snapshot. Only one build and one installation session may be
active. Rotation retains the running build. If Android kills the host during a
build, the next launch reports interruption and permits a fresh build. Installer
session IDs and results persist; if a confirmation is lost after process death,
use **Cancel install**, then retry. There is no silent installation or automatic
uninstall on signing conflicts.

## Canonical project format

An `.aicproject` file is a ZIP containing exactly:

- `project.properties`: UTF-8 Java properties with `formatVersion=1`, `name`,
  `profile=android-35`, and `optLevel=0|1`.
- `source.aic`: the exact UTF-8 source, including whitespace and Unicode.

Display names contain 1–120 printable characters. Source is limited to 1 MiB,
metadata to 16 KiB, and the input archive to 2 MiB. Unknown/duplicate archive
entries, paths, unsupported versions/settings, oversized expansion, and invalid
UTF-8 are rejected. Private storage assigns a UUID and uses `AtomicFile` updates.
Archives contain no local IDs, paths, APKs, caches, or keys. Timestamps in ZIP and
properties serialization are not canonical project data; round-trip equality is
defined by the four `ProjectData` values.

Import/export transfers source and settings, not signing identity. Import into
another host installation cannot update an existing app signed by the old key.
Keep that distinction in mind before clearing host storage. Project export also
does not export the generated app's own SQLite/SharedPreferences data.

## Tests

```powershell
cargo test --manifest-path compiler/Cargo.toml --workspace --locked
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets --locked -- -D warnings
./host/gradlew.bat -p host testDebugUnitTest lintDebug
python host/scripts/device-m6.py --instrument --verify
python host/scripts/device-m6.py --ui
```

The instrumented test APK is separate from the shipped host. It runs JNI,
packaging, signing, project storage and recreation tests inside the host UID.
The verifier pulls eight device-built APKs and compares DEX/manifest bytes with
the desktop CLI, then runs official dexdump, zipalign and apksigner checks. The
UI driver uses ordinary host controls and Android confirmation dialogs. It does
not bypass install-source settings or submit apps to external scan services.
Output is under ignored `compiler/testdata/generated/m6/`.

See [ADR 0004](adr/0004-m6-android-host.md) for the bootstrap/M8 boundary and
[acceptance evidence](m6-acceptance-evidence.md) for actual results.
