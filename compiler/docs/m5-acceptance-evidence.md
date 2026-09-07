# M5 acceptance evidence

## Automated gates

- Workspace unit/integration/doc tests pass, including focused reachability, O0 identity, constant folding, persistence-resource pruning, DEX pool pruning, deduplication, and deterministic-output tests.
- The Android 35 build and packaging pipeline succeeds independently at O0 and O1.
- Raw size results are stored in `testdata/generated/m5/size-results.csv` and `.json`.

## Measured result

On 2026-09-06, the representative interactive M5 fixture produced:

| Metric | O0 | O1 | Change |
| --- | ---: | ---: | ---: |
| `classes.dex` | 4,476 bytes | 3,100 bytes | -1,376 bytes (-30.7%) |
| aligned unsigned APK | 5,516 bytes | 4,140 bytes | -1,376 bytes (-24.9%) |
| signed APK | 12,691 bytes | 12,691 bytes | unchanged because signing-block output dominated this small fixture |

The strict primary gate passes because O1 DEX and unsigned APK are smaller while structural/differential tests preserve behavior. No inlining or specialization optimization is retained without measurement.

## Device gate

On 2026-09-06, `scripts/device-smoke-m5.ps1 -SkipBuild` passed at O0 and O1 on the authorized `arm64-v8a` reference device (`2312DRA50G`, Android 16). Both APKs displayed `Count: 1` after the fixed interaction and had clean Android crash buffers. Per-level raw evidence is written beside the generated APKs.

The corrected 10-sample performance run completed on the same device. O1 median cold start was 362 ms versus 382.5 ms for O0 (5.4% faster). Fixed-interaction wall time was 2,635 ms versus 2,607 ms (1.1% difference). O1/O0 total PSS was 82,861/82,527 KB (0.4% difference), and total RSS was 206,268/205,852 KB (0.2% difference). Both levels reported 7 views, 1 activity, 80 bitmap allocations totaling 4,866 KB, and 426 other native allocations totaling 39 KB. These small memory/runtime differences are treated as device noise, not optimization claims.

Raw cold-start samples, summaries, device identity, allocation-method disclosure, and complete `dumpsys meminfo -d` output are stored under `testdata/generated/m5/`. The deterministic size improvement remains the declared primary M5 win; the device results show no material performance, memory, or allocation regression.
