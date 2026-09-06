# M3 acceptance evidence

Recorded 2026-09-06.

## Passed

- `cargo fmt --all --check`
- `cargo test --workspace` (all unit, integration, and documentation tests)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `scripts/build-m3.ps1` for counter and calculator at optimization levels 0 and 1
- Independent `dexdump` inspection: both activities implement `View.OnClickListener`, expose generated `onClick(View)`, and contain deterministic field tables
- `apksigner verify --verbose`: counter and calculator O1 APKs verify with v1, v2, and v3 signatures
- Physical ARM64 device installation and interaction tests at optimization levels 0 and 1
- Counter increment, decrement, reset, and recreation-to-initial-state behavior
- Calculator add, subtract, multiply, divide, divide-by-zero, and out-of-range input behavior
- Clean Android crash buffer for all four generated APK executions

## Device gate

`scripts/device-smoke-m3.ps1 -SkipBuild` completed successfully against the authorized ARM64 reference device. Per-fixture evidence is stored beside the ignored generated artifacts under `testdata/generated/m3/`.
