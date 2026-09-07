# ADR 0004: M6 Android host and bootstrap boundary

Status: accepted

M6 adds the `dev.aic.host` Kotlin application, targeting Android 35 and supporting
ARM64 devices from API 30. Generated applications retain the existing
`android-35` profile (DEX 035, minSdk 23, targetSdk 35). API 30 is the minimum of
the selected Android AAPT2 port, not a change to generated-app requirements.

The host is built with Gradle 8.14.3, AGP 8.10.1, Kotlin 2.1.20 and JDK 17 or
newer. Rust is cross-compiled with NDK 27.1.12297006. Java/Kotlin/Gradle are used
only to build the host; generated applications use the embedded Rust compiler.

The user selected bootstrap tooling for M6. The ARM64 AAPT2 port is built from
ReVanced/aapt2 v1.1.0, commit
`adb1d7acef11a67849b8a6204a824e271c4c1952`, including pinned AOSP submodules and
upstream patches. `host/scripts/prepare-aapt2-source.py` makes source symlinks
usable on Windows and adjusts Expat's include path. Static NDK zlib is explicitly
selected. The executable ships as `libaapt2.so` in the host APK, is extracted by
Android's package manager, and executes from `nativeLibraryDir`, never writable
project storage. The host bundles the pinned Android 35 `android.jar`.

`aic-build` extracts the desktop compiler orchestration and ZIP assembly. Its
only packaging addition is a checked ZIP extra field aligning the stored DEX to
four bytes. This preserves the existing resource packaging bootstrap boundary;
it is not a general resource compiler or a new signing implementation. The CLI
retains its commands, textual manifest/profile contract and generated DEX.
Unsigned ZIP bytes may differ from pre-M6 output because DEX alignment is now
performed during assembly rather than solely by desktop zipalign.

AOSP apksig 8.10.1 runs in-process using Android's standard cryptographic
providers and a private Android Keystore RSA key. The signer emits v1 and v2
signatures, explicitly disabling v3/v4. No private key is exported or bundled.
The host build key is a separate ignored desktop debug keystore. Clearing host
data/uninstalling it can lose the generated-app identity; project import alone
does not transfer it. Existing APKs signed by another identity cannot be updated
by this host. The host reports the platform rejection without uninstalling data.

The compiler core still forbids unsafe Rust. `aic-jni` is the only FFI boundary:
it exports JVM entry points, uses the `jni` crate's reference and string wrappers,
and catches Rust unwinding before returning across the C ABI. There are no
handwritten unsafe blocks. JNI results contain `ok` and either artifact identity
and optimization report, or source-located diagnostics. Files go into a fresh
host-controlled build directory; signing keys never enter the Rust interface.

M8 follow-ups: replace the AAPT2/platform resource dependency, retire bootstrap
packaging where justified, evaluate signing/toolchain independence, and expand
the Android compatibility matrix. M6 does not claim M8 completion.
