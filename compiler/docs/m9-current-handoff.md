# M9 current handoff

Updated: 2026-09-14 (Asia/Taipei)

## Current state

- M9 remains open. M9.1 and M9.3 are implemented; M9.2, M9.4, and M9.6 are partial; M9.5 has not started; M9.7 remains blocked by implementation and matrix prerequisites.
- M9.3 is complete. The user explicitly waived manual TalkBack checks on non-hardware API profiles; API 35 tablet/compact automated accessibility evidence and the full API 36 physical-device TalkBack pass are retained.
- M9.2 typed start-for-result/results and Bundle state restoration remain separate open dependencies for milestone closure. Do not switch to them while continuing the selected M9.4 task unless explicitly requested.
- No new capability was added. Compiler and host `aic.capabilities/0.2` catalogs remain identical; project assets, lifecycle restoration, localization/resources, images, and adaptive UI remain `planned`.

## Uncommitted implementation to preserve

The worktree intentionally contains the M9.3 item 1 implementation and evidence. Do not reset, discard, or overwrite it.

- Expanded `schema/aic-ir-0.2.ebnf` for the complete declared UI syntax.
- Added `schema/m9-ui-capability-evidence.tsv` and a regression test requiring every supported `ui.*` and `accessibility.*` capability to have grammar, verifier, optimizer, lowering, invalid-input, and runnable evidence.
- Added focused verifier fixtures for invalid padding, gravity, visibility, and built-in icon values.
- Added optimizer preservation and aggregate direct DEX surface tests.
- Updated `testdata/m9-navigation.aic`: `DetailActivity` now uses a platform `ScrollView` with exactly one vertical `LinearLayout` child.
- Updated the root milestone, production-navigation status, and acceptance-evidence ledger.

Run `git status --short` before any edit. The expected changed paths are the root milestone plus the M9 IR/optimizer/DEX tests, EBNF, fixture, four new invalid fixtures, evidence inventory, and M9 documentation.

## Verification already completed

- Compiler workspace tests: passed.
- Strict workspace Clippy with `-D warnings`: passed.
- Host JVM tests and lint: passed.
- Complete offline M8 prerequisite and M9 build: passed.
- Compiler/host capability catalog parity and `git diff --check`: passed.
- Two O0 and two O1 fixture compilations were deterministic. Executable artifacts were identical across both optimization levels:
  - `classes.dex`: `CB3996F129EB8D0B49D250DB30837BDF657BE1C86314862774DB1438D8F3291E`
  - `classes2.dex`: `94952AC4CC38362690E5A80C3609887D8AC04655C41A45E6DC36CEC776978637`
  - `classes3.dex`: `0BA7E04B1E81B11F931B867219714D4EBE6895F8FEDC369EB7AFA1CC242F3FC6`
  - unsigned APK: `C68480C1C18DDCB5F4809AB85EC41C79675EA37C6B30590CE773838B560F0625`

Toolchain: source base `6f15fd41174ed72e18b6c308d316eecd42cf9cdf` plus the current worktree; `rustc 1.98.1 (48a229cea 2026-09-01)`; `cargo 1.98.1 (797e8a9bc 2026-08-05)`.

## Device evidence

- The user reported every supplied regression check OK at both O0 and O1, including the new Details `ScrollView`, navigation, controls, rotation/resume, enlarged font, TalkBack semantics, and final cold launch.
- Device: serial `e56c4a46`, model `2312DRA50G`, ABI `arm64-v8a`, Android 16/API 36.
- Fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`.
- O0 and O1 signed APK SHA-256: `10B169EB01D8B300210D03F74EE8A2BC379B787EAD2736C1DF902A8B8FE6CE9B`.
- ADB pull confirmed the installed base APK has the same hash. Build copies are under ignored `compiler/target/m9-device-o0` and `compiler/target/m9-device-o1` and can be rebuilt if absent.
- API 35 AVD `AIC_M9_API35_Tablet` (`emulator-5554`, Pixel Tablet, native x86_64) passed the automated O0/O1 matrix at native 2560x1600/320dpi and temporary compact 720x1280/240dpi profiles. Rotation, background/resume, 1.3 font scaling, widget/navigation/input/selection behavior, 48dp floors, installed-hash parity, screenshots, and crash buffers passed.
- API 35 fingerprint: `google/sdk_gtablet_x86_64/emu64xa:15/AE3A.240806.046.T1/13135149:user/dev-keys`. O0/O1 signed and installed APK SHA-256: `C90F37FAAD8FB476466D853C71D03BB060880979E75D8CA57B9A767BC12827B0`.
- TalkBack was installed and its service could be enabled on API 35. Spoken announcements and touch exploration were not manually confirmed on the emulator and were explicitly waived by the user on 2026-09-15. Emulator display/density/font/accessibility overrides were restored after automation.
- The API 35 tablet/compact O0/O1 automated matrix was repeated successfully on 2026-09-15. The AVD was then stopped and deleted as requested to minimize writable emulator storage; the installed API 35 system image and ignored evidence under `compiler/target/m9-matrix` remain.
- On 2026-09-15 the user completed the full physical-device checklist against the newly signed representative-matrix artifact. Rendering, controls, menu/dialog, navigation, `ScrollView`, inputs, selection, rotation/resume, enlarged font, the complete TalkBack scenario, restored normal touch, and final cold-launch stability all passed. The signed and previously ADB-pulled installed APK SHA-256 was `C90F37FAAD8FB476466D853C71D03BB060880979E75D8CA57B9A767BC12827B0`. This refreshes the API 36 physical row but does not satisfy the distinct API 35 manual TalkBack rows.

## Exact next task

Continue with M9.4. Begin with the canonical resource declaration/model and deterministic resource-ID/package foundation needed by default and BCP-47 localized strings, then add project-image references and host-to-compiler asset transfer. Keep `resources.localization`, `resources.images`, and `ui.image.project_asset` planned until their parser, verifier, packaging/lowering, invalid-fixture, capability, deterministic O0/O1, and AAPT2-oracle evidence is complete.
