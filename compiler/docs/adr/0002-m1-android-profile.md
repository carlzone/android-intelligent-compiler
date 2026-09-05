# ADR 0002: M1 Android profile and signing

Status: accepted

M1 emits DEX 035 for `minSdkVersion` 23 and `targetSdkVersion` 35, links manifests against Android platform 35, and pins Android build-tools 35.0.0. Generated activities extend `android.app.Activity` and construct classic framework Views programmatically. AppCompat, Compose, resource layouts, and third-party runtime libraries are excluded.

AAPT2, zipalign, apksigner, keytool, and ADB are bootstrap/reference tools. The compiler itself writes `classes.dex`, the text manifest, and the APK ZIP entry for the DEX. Local debug keys live under ignored `compiler/.aic/`; callers may supply a different debug keystore. Private keys are never repository fixtures.

Unsigned compiler products are deterministic. Signed APK bytes are outside the reproducibility guarantee because signing material and metadata vary. M1's reference device is the sole connected authorized device whose primary ABI begins with `arm64`; the smoke test records its serial, model, ABI, Android release, and build fingerprint.
