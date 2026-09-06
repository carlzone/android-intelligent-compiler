# M2 acceptance evidence

Status: **COMPLETE**  
Recorded: 2026-09-06  
AIC IR: `0.1`  
Android profile: API 35, DEX 035, minSdk 23

## Toolchain and reference device

- Rust: `rustc 1.98.1`, `cargo 1.98.1`
- Device: Xiaomi/Redmi `2312DRA50G`
- ABI: `arm64-v8a`
- Android: 16, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`

The compiler continues to target the documented Android 35 profile even though the reference runtime device is API 36.

## Verified gates

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Valid fixtures compile twice at O0 and O1 with byte-identical unsigned `classes.dex` output.
- Invalid immutable-assignment, type-mismatch, and recursion fixtures fail before DEX output with `AIC1108`, `AIC1116`, and `AIC1119` respectively.
- Structural tests prove runtime methods, calls, branches, object instructions, reference returns, and the absence of complete precomputed output strings.

## Physical-device matrix

| Fixture | O0 | O1 | Exact visible output |
| --- | --- | --- | --- |
| Bounded-loop computation | PASS | PASS | `Result: 36` |
| Dynamic string/reference return | PASS | PASS | `Value: 7` |
| Chained multi-function calls | PASS | PASS | `Answer: 22` |
| String equality/inequality | PASS | PASS | `Equal: true, different: true` |
| AND/OR short-circuit safety | PASS | PASS | `Short circuit: false,true` |

Every run installed the signed APK, cold-launched its generated Activity, matched the exact visible text, and produced no fatal crash-buffer entry. The matrix was repeated successfully after the Java platform boolean descriptors were corrected to `Z`.

## Deferred features

Floating-point and decimal types, recursion, unbounded loops, `break`, `continue`, and the M3 UI/event surface remain explicitly outside M2.
