# M4 acceptance evidence

## Automated gates

- Rust formatting, workspace tests, and Clippy with warnings denied.
- Notes compilation and deterministic DEX checks at optimization levels 0 and 1.
- Independent `dexdump`, manifest, and `apksigner` checks performed by `build-m4.ps1` and the existing packaging pipeline.
- Generated notes manifests contain no permissions.

## Device gate

Run `scripts/device-smoke-m4.ps1 -SkipBuild` against the authorized ARM64 reference device. The script records per-optimization-level evidence for create/read/update/delete, force-stop/relaunch persistence, the preference-backed last selected ID, invalid/missing records, and the Android crash buffer under `testdata/generated/m4/`.

On 2026-09-06 both O0 and O1 APKs passed create/read/update/delete, force-stop/relaunch persistence, preference restoration, missing-record and invalid-ID handling, and crash-buffer checks on the authorized ARM64 reference device. Their builds, DEX inspections, permission-free manifest inspections, and signature verifications also passed.
