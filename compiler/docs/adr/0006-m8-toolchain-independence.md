# ADR 0006: M8 self-contained supported APK pipeline

Status: accepted implementation; runtime exit gate open.

The generated-app path now runs entirely in-process: verified AIC IR -> optimized
DEX -> compiler-owned manifest model -> binary XML -> aligned stored ZIP -> AOSP
apksig 8.10.1 -> Android Keystore v1/v2 signatures. AAPT2 and android.jar are no
longer shipped or invoked by the host. SDK/NDK/Gradle remain host-development
requirements, and official tools remain independent desktop oracles.

`aic-res` owns the deliberately narrow Android-35 manifest model and writer.
Both textual and binary output derive from that model. The profile pins DEX 035,
minSdk 23, targetSdk 35, the framework Material Light NoActionBar theme, a single
exported launcher Activity, and literal labels. Current verified persistence
capabilities require no manifest permissions. Unsupported capabilities continue
to fail before lowering. No resource table is necessary; app resource tables,
layouts, icons, themes and localization expansion remain M9 work.

Framework IDs are checked against Android 35 android.R using javap in the
acceptance runner. XML uses a deterministic UTF-16 string pool (including long
length encoding), resource map and typed attributes. XML 1.0 forbidden characters
are rejected. Chunk/string sizes and index narrowing are checked. ZIP entries
are stored in manifest/DEX order, four-byte aligned, and dated 1980-01-01 00:00.
ZIP64, multidisk archives and duplicate appended entries remain unsupported.
The legacy bootstrap ZIP-injection CLI is retained for oracle workflows.

Retain AOSP apksig rather than write a new signer: it already executes within the
host and delegates cryptography to Android. The user selected this boundary.
The generated debug key alias `aic-generated-debug-v1` and host debug key are
unchanged, as are installation approval and signing-conflict behavior. v3/v4,
release identity migration and Play distribution remain out of scope.

Compile results gain binary manifest and unsigned APK bytes; CLI/JNI persist
AndroidManifest.axml and unsigned.apk alongside existing outputs. JNI no longer
exports the obsolete AAPT2-base assembly method. Project and AI wire formats do
not change. Packaging errors use AIC8001/AIC8002 and host signing uses AIC8003.

The compatibility gate is every generated-app API 23-36 and every ARM64 host API
30-36, with offline host builds. Official signature verification across an API
range is not runtime compatibility evidence. Missing rows keep M8 open.
