# M9 acceptance evidence

Status: implementation in progress; exit gate open.

New threads should read `m9-current-handoff.md` first for the concise worktree,
verification, device, and resume snapshot.

## Phase evidence ledger

| Phase | Implementation evidence | Acceptance evidence | Status |
| --- | --- | --- | --- |
| M9.1 IR/migration/catalog | Parser/verifier migration tests, CLI migration, compiler/host catalog parity | Full 0.1 behavior and APK reproducibility corpus remains part of final run | Implemented; final rerun pending |
| M9.2 screens/navigation/state | Multi-Activity DEX/manifest tests and start/finish fixture; typed Intent extras parser/verifier/lowering | Results, Bundle restoration, and device lifecycle evidence missing | Active / partial |
| M9.3 UI/widgets/accessibility | The complete declared surface is mapped to grammar, verifier, optimizer, lowering, invalid-input, and runnable evidence; `m9-navigation.aic` also covers valid single-child ScrollView composition | API 36 ARM64 physical-phone checks pass at O0/O1; API 35 x86_64 tablet at 2560x1600/320dpi and compact 720x1280/240dpi automated checks pass at O0/O1; manual TalkBack on non-hardware API profiles was explicitly waived | Complete with documented emulator-accessibility limitation |
| M9.4 resources/localization/assets | Project format 2 codec and asset-validation tests | Resource table/package, locale, image build, and AAPT2 evidence missing | Partial |
| M9.5 adaptive/lifecycle corpus | None yet for the complete declared behavior | Rotation, process death, locale, font, TalkBack, density, and size matrix missing | Not started |
| M9.6 host/AI/diagnostics/reference apps | IR 0.2 contract/templates, initial fixture, ADR, and build script | Final response schema and three-app corpus missing | Partial |
| M9.7 acceptance/closure | Local tests have passed during implemented tranches | Full fingerprinted desktop/device run and M8 matrix closure missing | Blocked |

M9.3 is complete as of 2026-09-15. The representative matrix is the API 36 ARM64 480dpi physical phone plus API 35 x86_64 tablet and compact emulator profiles. The user explicitly waived manual TalkBack confirmation on non-hardware API profiles; API 35 retains automated accessibility-node/service evidence, while the complete manual TalkBack scenario passed on API 36 hardware. Exhaustive API 23-36 generated-app and API 30-36 native ARM64 compatibility remains an M8/M9.7 gate. The current milestone resume point is M9.4 canonical resources and deterministic APK resource packaging.

## Representative API 35 matrix evidence — 2026-09-14

The installed `system-images;android-35;google_apis_playstore_tablet;x86_64` image was configured as AVD `AIC_M9_API35_Tablet`. It ran natively as x86_64; its advertised ARM64 translation support is not counted as native ARM64 evidence. The same AVD supplied two non-duplicative layout profiles: its native 2560x1600, 320dpi tablet configuration and a temporary 720x1280, 240dpi compact configuration. Display, density, rotation, font-scale, and accessibility-service settings were restored after collection.

The complete four-row automated matrix was repeated successfully on 2026-09-15 using the running AVD. After the tablet O0/O1 and compact O0/O1 results, all overrides were restored, the emulator was stopped, and `AIC_M9_API35_Tablet` was deleted to release its writable AVD storage. The API 35 system image and ignored evidence artifacts were retained.

At both O0 and O1, each profile passed cold launch; widget presence; CheckBox and Switch toggling; collection selection; Details navigation and platform `ScrollView`; hidden-control exclusion; informative-image semantics; all declared input fields; text entry and password-node behavior; Spinner setup/selection; orientation recreation; background/resume; 1.3 font scaling; density-derived minimum touch heights; installed-APK hash parity; and a clean Android crash buffer. Large-font screenshots were inspected for the intended pale-blue background, dark heading text, readable controls, and absence of overlap. The tablet controls measured at least 96px at 320dpi and compact controls at least 72px at 240dpi, both exactly representing the 48dp floor.

- Emulator serial/model: `emulator-5554`, `Pixel Tablet`
- Android/API/ABI: Android 15, API 35, `x86_64` (`arm64-v8a` translation advertised but not counted)
- Fingerprint: `google/sdk_gtablet_x86_64/emu64xa:15/AE3A.240806.046.T1/13135149:user/dev-keys`
- Native tablet profile: 2560x1600, 320dpi, portrait/landscape exercised, O0/O1 passed
- Compact override profile: 720x1280, 240dpi, portrait/landscape exercised, O0/O1 passed
- O0/O1 signed and installed APK SHA-256: `C90F37FAAD8FB476466D853C71D03BB060880979E75D8CA57B9A767BC12827B0`
- Executable hashes: `classes.dex` `CB3996F129EB8D0B49D250DB30837BDF657BE1C86314862774DB1438D8F3291E`; `classes2.dex` `94952AC4CC38362690E5A80C3609887D8AC04655C41A45E6DC36CEC776978637`; `classes3.dex` `0BA7E04B1E81B11F931B867219714D4EBE6895F8FEDC369EB7AFA1CC242F3FC6`; unsigned APK `C68480C1C18DDCB5F4809AB85EC41C79675EA37C6B30590CE773838B560F0625`

TalkBack package `com.google.android.marvin.talkback` was installed and its service was successfully enabled on API 35. Automated hierarchy evidence confirms the informative image description, decorative-image omission, progress description, hidden-control exclusion, and functional traversal targets. Headless automation cannot truthfully establish spoken announcements or subjective touch-exploration quality. On 2026-09-15 the user explicitly waived those manual checks on non-hardware API profiles; the limitation is retained in this evidence rather than represented as a manual pass.

### Refreshed API 36 physical-device pass — 2026-09-15

The user completed the full supplied checklist on the physical API 36 ARM64 phone after the representative-matrix O1 artifact was installed and cold-launched. Every reported item passed: basic rendering and colors; edge/corner activation; collection selection with exactly-once heading updates; popup menu, dialog, Details `ScrollView`, close/back navigation, and hidden-control exclusion; all five input modes and password masking; Spinner setup suppression and explicit selection; rotation on every Activity; background/resume; enlarged-font layout and restoration; TalkBack headings, descriptions, decorative-image omission, label association, traversal, touch exploration, and double-tap activation; normal touch after disabling TalkBack; and final force-close/cold-launch stability without a crash or ANR.

- Device serial/model/ABI: `e56c4a46`, `2312DRA50G`, `arm64-v8a`
- Android/API: Android 16, API 36
- Fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- Installed optimization artifact: O1 (byte-identical to O0)
- Signed and ADB-pulled installed APK SHA-256: `C90F37FAAD8FB476466D853C71D03BB060880979E75D8CA57B9A767BC12827B0`

This refreshes and fully passes the physical-phone row for the current representative-matrix artifact. Together with the passing API 35 automated rows and the explicit non-hardware TalkBack waiver, it closes M9.3.

## Declared UI family audit — 2026-09-13

The item 1 audit passed workspace tests, strict Clippy, host JVM tests and lint, the complete offline M8 prerequisite build, compiler/host catalog parity, focused parser/verifier/optimizer tests, and aggregate direct DEX inspection. The IR 0.2 EBNF now enumerates the complete supported widget/property syntax. Focused valid and invalid cases cover bounded padding, start/center/end alignment, visible/invisible/gone visibility, info/warning/delete icons, inline collections, containment, and the existing accessibility constraints. The runnable fixture's Details screen now uses a platform `ScrollView` with exactly one linear-layout child while preserving its existing controls and navigation.

Two compilations at both O0 and O1 were deterministic. All four runs produced identical executable artifacts: `classes.dex` SHA-256 `CB3996F129EB8D0B49D250DB30837BDF657BE1C86314862774DB1438D8F3291E`, `classes2.dex` `94952AC4CC38362690E5A80C3609887D8AC04655C41A45E6DC36CEC776978637`, `classes3.dex` `0BA7E04B1E81B11F931B867219714D4EBE6895F8FEDC369EB7AFA1CC242F3FC6`, and unsigned APK `C68480C1C18DDCB5F4809AB85EC41C79675EA37C6B30590CE773838B560F0625`. O0 and O1 build-profile metadata differs only as expected for the selected optimization level. The source base was `6f15fd41174ed72e18b6c308d316eecd42cf9cdf` plus this M9.3 audit worktree, using `rustc 1.98.1 (48a229cea 2026-09-01)` and `cargo 1.98.1 (797e8a9bc 2026-08-05)`. This closed implementation item 1; the completed representative device evidence is recorded above and below.

### Declared UI family audit O0/O1 device evidence — 2026-09-14

The user completed the supplied O0 and O1 checklist and reported every scenario OK on the connected physical device. Coverage included cold launch; Home widgets and typed selection behavior; popup menu and alert dialog; Details navigation and the new vertical `ScrollView` composition; reachability and activation of Details controls after scrolling; all common input modes, password masking, and Spinner setup suppression/selection; back-stack behavior; rotation and background/resume; enlarged-font layout; TalkBack headings, labels, informative/decorative image handling, progress description, and hidden-control exclusion; and final cold-launch stability.

- Device serial/model/ABI: `e56c4a46`, `2312DRA50G`, `arm64-v8a`
- Android/API: Android 16, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- O0 signed APK SHA-256: `10B169EB01D8B300210D03F74EE8A2BC379B787EAD2736C1DF902A8B8FE6CE9B`
- O1 signed APK SHA-256: `10B169EB01D8B300210D03F74EE8A2BC379B787EAD2736C1DF902A8B8FE6CE9B`
- Installed base APK SHA-256 confirmed through ADB: `10B169EB01D8B300210D03F74EE8A2BC379B787EAD2736C1DF902A8B8FE6CE9B`

The identical O0/O1 signed hashes are expected because the supported optimizer makes no byte-changing transformation to this verified fixture. This completes the single-device regression evidence for item 1; it does not close item 2's wider TalkBack, density, screen-size, or supported-device matrix.

The literal-color tranche passed workspace tests, strict Clippy, host JVM tests and lint, the complete offline M8 prerequisite build, compiler/host catalog parity, focused parser/verifier/optimizer/DEX coverage, and two deterministic compilations at both O0 and O1 on 2026-09-12. The runnable fixture uses dark `#202124` heading text on a deliberately visible but soft `#E8F0FE` MainActivity background. All four builds were byte-identical: `classes.dex` SHA-256 `CB3996F129EB8D0B49D250DB30837BDF657BE1C86314862774DB1438D8F3291E`, `classes2.dex` `CDC817AB2BDB1C4E862BB1C3EAADFA93EF83A67359FEB3AE2F289E9F8A37F488`, `classes3.dex` `0BA7E04B1E81B11F931B867219714D4EBE6895F8FEDC369EB7AFA1CC242F3FC6`, unsigned APK `C00ADB06EE0DF49C92F0FF3DD2E3865D3E694B1E3B104C01E2441B2A56846F8C`, and signed APK `20E53E97200D74163A1173436E8C16EE5BD957D440828E8E969466EA5A88E7CD`. The O0 and O1 signed APKs passed v1/v2 signature verification for the API 23–36 range. The source base was `0bd9e0531a01f34e97691d1c79303a1f8ef264a4` plus the recorded uncommitted M9 worktree, using `rustc 1.98.1 (48a229cea 2026-09-01)` and `cargo 1.98.1 (797e8a9bc 2026-08-05)`.

The first O0 device pass completed every regression item except visual confirmation of the original `#FAFAFA` background, which was too subtle to distinguish reliably by eye. That inconclusive value was replaced with `#E8F0FE`, rebuilt, signed, installed, and cold-launched. The user-supplied screenshot confirms the revised pale-blue MainActivity background and dark `#202124` heading text render correctly. The byte-identical O1 signed APK was then installed and cold-launched, and the user confirmed the same colors, selection text-color retention, Details/Input navigation, menu, dialog, back behavior, rotation, and final cold launch without crashes or rendering corruption. The complete color and regression checklist therefore passes at O0 and O1 on device `e56c4a46` (`2312DRA50G`, Android 16/API 36).

The accessibility-semantics tranche passed workspace tests, strict Clippy, host JVM tests and lint, the complete offline M8 prerequisite build, catalog parity, focused parser/verifier/optimizer/DEX coverage, and two deterministic compilations at both O0 and O1 on 2026-09-12. O0 and O1 were byte-identical for this fixture: `classes.dex` SHA-256 `3768C7F8DF9230A683DAE8375DBB3BCEEEB0BBA0FAE188D7B769E29901E98A04`, `classes2.dex` `CDC817AB2BDB1C4E862BB1C3EAADFA93EF83A67359FEB3AE2F289E9F8A37F488`, `classes3.dex` `0BA7E04B1E81B11F931B867219714D4EBE6895F8FEDC369EB7AFA1CC242F3FC6`, unsigned APK `FAEC6D7D147AB5B39673A00A925C733C6AEF388A0FDF09CD0C2BC33F8713E84E`, and signed APK `87FC90BE8474266585A5D191E0642C60A22523E11FC294EB000A110DA54EB2A9`. The source base was `0bd9e0531a01f34e97691d1c79303a1f8ef264a4` plus the recorded uncommitted M9 worktree, using `rustc 1.98.1 (48a229cea 2026-09-01)` and `cargo 1.98.1 (797e8a9bc 2026-08-05)`.

### Accessibility-semantics O0/O1 device staging — 2026-09-12

The signed O0 APK was installed first with update semantics and cold-launched successfully. The signed O1 APK was subsequently installed with update semantics and cold-launched successfully after the O0 testing stage. The user completed the full checklist at both optimization levels. TalkBack announced the progress control as `Loading details`, omitted the decorative image, and announced the informative Details image as `Information`. The hidden disabled control was neither visible nor reachable. Heading navigation, popup menu, dialog, forward/back/finish navigation, rotation on every Activity, enlarged-font layout and restoration, normal touch after disabling TalkBack, and crash checks all passed at O0 and O1. O1 `MainActivity` remained top-resumed with a clean crash buffer.

- Device serial/model/ABI: `e56c4a46`, `2312DRA50G`, `arm64-v8a`
- Android/API: Android 16, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- Installed optimization level: O1
- Installed signed APK SHA-256: `87FC90BE8474266585A5D191E0642C60A22523E11FC294EB000A110DA54EB2A9`

The minimum-touch-target tranche passed workspace tests, strict Clippy, host JVM tests and lint, compiler/host catalog parity, focused parser/verifier/optimizer/DEX coverage, and two deterministic compilations at both O0 and O1 on 2026-09-12. All four runs produced identical Activity DEX units: `classes.dex` SHA-256 `00BD7B6775A06E3742FDB0A7F4C764511911E024BA065AD736F996F551179CEE`, `classes2.dex` `00B7A6EC1D572A592B10DEC489431DBBE01DEE8F613650964558241AEC1ADA57`, and `classes3.dex` `827F31BB591163BCDA3D83655FB4B02771E94ADCB04884DA51985F05742C9DB6`. The source base was `0bd9e0531a01f34e97691d1c79303a1f8ef264a4` plus the recorded uncommitted M9 worktree, using `rustc 1.98.1 (48a229cea 2026-09-01)` and `cargo 1.98.1 (797e8a9bc 2026-08-05)`. The local evidence is supplemented by the completed O0/O1 device scenario below.

### Minimum-touch-target O0/O1 device evidence — 2026-09-12

The user completed all 15 checklist items at both O0 and O1 on the connected API 36 ARM64 device. At the device's 480dpi density, the toolbar, Open Details button, CheckBox, Switch, and visible ListView row measured 144px high, exactly 48dp. Edge/corner activation, popup menu, alert dialog, forward/close/input navigation, CheckBox and Switch toggling, all three vertically scrollable ListView rows, all five input fields and keyboard modes, password masking, Spinner choices and popup rows with exactly-once updates, rotation on every Activity, background/resume, enlarged-font layout, font restoration, and final cold launch passed at both optimization levels. The compact ListView viewport intentionally showed one row at a time. The initial Spinner `First` position correctly remained `No selection` because setup delivery is suppressed; selecting another item and returning to First delivered the expected update. Editable contents clearing on rotation remains the documented open M9.2 Bundle-restoration dependency and was not counted as an M9.3 failure. The final O1 MainActivity remained resumed and the crash buffer was clean.

- Device serial/model/ABI: `e56c4a46`, `2312DRA50G`, `arm64-v8a`
- Android/API: Android 16, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- O0 signed APK SHA-256: `8817B12C6A57ACC09C0FCA9C8A6A75243BC012932141E35551366BF6A2D4B671`
- O1 signed APK SHA-256: `8817B12C6A57ACC09C0FCA9C8A6A75243BC012932141E35551366BF6A2D4B671`

The label/heading tranche passed workspace tests, strict Clippy, host JVM tests and lint, compiler/host catalog parity, focused parser/verifier/optimizer/DEX coverage, and deterministic O0/O1 compilation on 2026-09-12. The three generated Activity DEX units were byte-identical across O0/O1: `classes.dex` SHA-256 `F84BA4F857D3990B29E254E4326602C77C6EDF7891723CE7B2E86DB340026C40`, `classes2.dex` `EAEEEDDDD23947DB3796F3A8F3A908EEFF5228545FD253FB65A1DF379011998D`, and `classes3.dex` `EF56AF72FE0AA5FD746C876EF6E339857BE3DD659778505BCBECEA2A23E7A18E`. The completed device and TalkBack validation is recorded below.

### Explicit input-label and semantic-heading device evidence — 2026-09-12

The user completed all 38 requested checks on the connected physical device. The O1 update installed and cold-launched successfully with a clean crash buffer. TalkBack associated the visible `Text` label with the first editable input without treating the label as editable; text entry and the expected keyboard remained functional. `Home` and `Details` were announced and discoverable through heading navigation, while ordinary text and buttons were not announced as headings. Popup menu, alert dialog, forward/back/finish navigation, rotation on all three Activities, background/resume, list and Spinner selection, input modes, password masking, checkbox, switch, enlarged-font layout, normal touch after disabling TalkBack, and final cold relaunch all passed without an incorrect announcement, blank screen, or crash.

- Device serial: `e56c4a46`
- Model/ABI: `2312DRA50G`, `arm64-v8a`
- Android/API: `16`, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- Source base: `0bd9e0531a01f34e97691d1c79303a1f8ef264a4` plus the recorded uncommitted M9 worktree
- Compiler toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`
- Installed O1 signed APK SHA-256: `B0FC85646892295009FED01E8B375A9A6D5DB20BD03DD0B6095F290281709683`

This closes the single-device TalkBack scenario for explicit labels and semantic headings. It does not close minimum touch-target enforcement or the wider density, screen-size, supported-device, and M9.7 matrices.

The text-sizing tranche passed workspace tests, strict Clippy, host JVM tests, catalog parity, focused parser/verifier/optimizer/DEX coverage, and repeated O0/O1 compilation on 2026-09-12. Both O0 runs and the O1 run produced `classes.dex` SHA-256 `2AC81F37597539414D838DF0EF5D8CC4872C9048312C75D3A4FABB2B31756888`.

## M9.3 device smoke feedback

User-reported physical-device smoke testing passed the Main/Details navigation flow, popup menu, alert dialog, back/finish behavior, existing widgets, fixed layout/margins, rotation, relaunch, and crash-buffer check. The initial common-input run found the first text field obscured by the status bar. Generated content roots were then updated to call platform `View.setFitsSystemWindows(true)` before `setContentView`; the user confirmed the corrected APK resolves the overlap. Text, email, password masking, phone, integer keyboards, and Spinner selection otherwise behaved as expected.

That earlier common-input feedback remains positive but non-fingerprinted evidence. The collection/selection scenario below is separately recorded with device, build, optimization, and artifact metadata.

### Typed collection and selection event device evidence — 2026-09-12

The user completed the collection/selection checklist at both O0 and O1 on the connected physical device. All 18 scenarios passed: launch; reusable ListView contents and row taps; observable zero-based row behavior; navigation to InputActivity; Spinner contents; suppression of the initial Spinner callback; selection of Second, Third, and return to First/index 0; exactly-once delivery; rotation before and after selection; recreation without crashes; background/resume; touch and available keyboard/D-pad/TalkBack interaction; existing navigation/menu/dialog/back behavior; and a clean listener-related crash check.

- Device serial: `e56c4a46`
- Model/ABI: `2312DRA50G`, `arm64-v8a`
- Android/API: `16`, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- Source base: `0bd9e0531a01f34e97691d1c79303a1f8ef264a4` plus the recorded uncommitted M9 worktree
- Compiler toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`
- O0 APK SHA-256: `10499603C82F513E9E5F5437D3E090CACD82C68A5AE833688C429E5A9618AD29`
- O1 APK SHA-256: `10499603C82F513E9E5F5437D3E090CACD82C68A5AE833688C429E5A9618AD29`

The identical O0/O1 hashes are expected for this fixture because the supported optimizer makes no byte-changing transformation to these verified activities. This closes the collection/selection tranche; final clean-source compatibility remains part of M9.7.

### Bounded scale-safe text sizing device evidence — 2026-09-12

The user completed the 21-item text-sizing checklist at both O0 and O1 on the same connected physical device. Every scenario passed: cold launch; visibly enlarged title; readable and unclipped Button, CheckBox, and Switch text; unchanged ListView contents and selection behavior; menu, dialog, navigation, finish/back, and all five input modes; enlarged text-input hint and initial `No selection` label; Spinner setup suppression and subsequent First/Second/Third updates; rotation on every Activity; background/resume; enlarged Android system font followed by relaunch; observable SP scaling without clipping, overlap, or off-screen text; restoration to the default font size; and a clean Logcat/crash-buffer check.

- Device serial: `e56c4a46`
- Model/ABI: `2312DRA50G`, `arm64-v8a`
- Android/API: `16`, API 36
- Build fingerprint: `Redmi/garnet_global/garnet:16/BP2A.250605.031.A3/OS3.0.4.0.WNRMIXM:user/release-keys`
- O0 signed APK SHA-256: `91F185E4A84ADB8A99624BAD7BB46674A0B54E4CB607F164985C13C3356526ED`
- O1 signed APK SHA-256: `91F185E4A84ADB8A99624BAD7BB46674A0B54E4CB607F164985C13C3356526ED`

The identical signed hashes confirm this fixture receives no byte-changing O1 transformation. This closes the tranche’s single-device default/enlarged-font scenario; the broader device, density, screen-size, TalkBack, semantic-heading, and touch-target matrix remains open.

Passing local evidence must include workspace tests, clippy, host JVM tests, and
deterministic compilation of `m9-navigation.aic` at O0 and O1. M9 cannot close before the existing M8 API 23–36 generated-app
and API 30–36 native ARM64 host matrix closes.
