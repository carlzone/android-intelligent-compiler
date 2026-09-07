# M5 whole-program optimization

M5 keeps AIC IR `0.1` and the existing `--opt-level 0|1` interface. O0 emits verified IR without transformation. O1 runs constant folding and dead-branch elimination, computes deterministic reachability from activity lifecycle roots, removes unreachable functions, and then removes preference, database, capability, SQL, method, prototype, and string-pool entries made unreachable by those transformations.

The analysis reports reachable functions, capabilities, views, preference keys, tables, and referenced columns. `optimize_with_report` returns the optimized program and report; the existing `optimize` API remains compatible. Build profiles contain deterministic before/after/removal counts. Initializers and visible views are retained because their evaluation or presence may be observable.

String, type, and prototype pools remain sorted and deduplicated. Function inlining and specialization are deferred: the implemented reachability passes already produce a measured reduction, and there is no benchmark evidence justifying the added complexity yet.

## Reproduction

From `compiler/`, run:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-m5.ps1
./scripts/device-smoke-m5.ps1 -SkipBuild
./scripts/benchmark-m5.ps1 -SkipBuild
```

`build-m5.ps1` builds identical source with O0 and O1, records exact DEX/aligned-APK/signed-APK bytes as CSV and JSON, and fails unless O1 DEX is strictly smaller. It uses the pinned Android 35/build-tools 35.0.0 profile and the same signing key for both levels.

The device benchmark requires exactly one authorized device. It performs one warm-up and ten force-stop cold starts per level, records raw `am start -W` `TotalTime` samples, and captures `dumpsys meminfo -d` after launch. Perfetto heapprofd is the preferred allocation method on a supported debuggable reference build; otherwise the run explicitly records that allocation totals are unavailable and retains heap/object counters as the fallback. Variable device metrics are reported rather than used as a noisy pass threshold.

## Behavioral contract

The optimizer preserves wrapping integer arithmetic, checked constant division/remainder behavior, short-circuit evaluation, function results, visible UI state, and persistence effects on reachable paths. Automated tests compare evaluator results, validate transitive call reachability, cover persistence-resource pruning, require deterministic reports/output, verify pool deduplication, and prove that unreachable methods and literals are absent from O1 DEX.
