# M9 production UI and navigation

M9 uses AIC IR 0.2 and the `aic.capabilities/0.2` catalog. Current implemented
surface includes deterministic 0.1 migration, multiple Activity declarations,
launcher/non-exported manifest generation, direct start/finish navigation and
typed `i32`/`bool`/`string` Intent extras,
multi-DEX packaging, padding, visibility, enabled state, content descriptions,
start/center/end alignment, literal text/background colors, nested frame layouts, checkboxes, switches, progress
indicators, and bounded platform icons.
The bounded menu/dialog group uses platform `Toolbar`, `PopupMenu`, and
`AlertDialog.Builder` calls without reflection or generated support libraries.
`android.list_view(items: [...])` accepts 1–100 typed string expressions and
lowers to a platform `ListView` plus `ArrayAdapter` using reusable standard rows.

The reference fixture is `compiler/testdata/m9-navigation.aic`. Build it with the
normal `aic-cli compile` path. Unsupported M9 catalog entries remain explicitly
`planned`; they must not be presented as supported by the host or AI layer.

M9 is not complete. Result passing, Bundle restoration, the remaining widget
groups, compiled resources/localization, adaptive variants, full accessibility
evidence, and the device matrix are still open.

## Execution phases and handoff checkpoint

Status terms are intentionally narrow: `Implemented` means code plus focused local tests exist, `Partial` means only part of the declared phase exists, `Not started` means no supported end-to-end capability exists, and `Blocked` means its exit work depends on earlier phases. No phase status by itself closes M9.

### M9.1 — IR version, migration, and capability contract — Implemented

- Accept IR 0.1 and canonical IR 0.2; expose deterministic `migrate --to 0.2` without rewriting stored projects.
- Publish matching compiler/host capability catalogs and report IR/catalog metadata through build-facing contracts.
- Parse and verify multiple named Activities while rejecting invalid 0.1 multi-Activity input, duplicate screens, and unknown targets.

Maintenance gate: migration determinism, catalog parity, and optimized/unoptimized compatibility tests remain green.

### M9.2 — Multi-screen lifecycle and typed navigation — Active / partial

Implemented: first Activity launcher selection, non-exported secondary Activities, one generated Activity class/DEX unit per screen, direct typed start, finish, and native back-stack behavior.

Next work, in order:

1. Define and verify typed `i32`, `bool`, and `string` Intent extras. **Implemented.**
2. Add typed start-for-result and result declarations with target/result compatibility diagnostics. **Next.**
3. Lower extra/result serialization and result delivery without reflection.
4. Save and restore declared screen state through `Bundle`, including recreation and process-death paths.
5. Add invalid fixtures, deterministic O0/O1 tests, independent DEX inspection, and a navigation/result device scenario.

Completion gate: navigation arguments/results and every declared screen-state type survive the documented lifecycle scenarios with source-located failures for invalid use.

### M9.3 — Production layout, widgets, and reusable UI — Partial

Implemented: linear/scroll/frame composition, system-window fitting for generated content roots, core sizing/weight behavior, padding, visibility, enabled state, alignment, text/button input, explicit text/email/password/phone/integer input modes, checkbox, switch, progress, built-in images, toolbar, popup menu, alert dialog, bounded platform spinners, and bounded literal string lists backed by platform reusable rows.

Implemented in the current M9.3 tranche: bounded `0..4096` fixed dimensions, optional four-sided margins on `android.set_layout`, containment checks for single-parent ownership, cycles, self-parenting, root ownership, and the single-child `ScrollView` rule, fixed state-backed `string[]` collections and typed `on_select(view, index, value)` events for ListView and Spinner, plus bounded `1..200` SP text sizing for platform TextView subclasses. Selection indices are zero-based and the initial Spinner framework callback is suppressed.

Explicit `android.set_input_label(label: ..., input: ...)` relationships and `android.set_heading(view: ...)` semantics are also implemented. Labels require a `text_view` and an `edit_text`/`text_input`; lowering assigns the input a generated platform ID and calls `TextView.setLabelFor`. Headings require a `text_view` and call `View.setAccessibilityHeading(true)` behind an API 28 runtime guard, preserving the API 23 minimum without reflection.

Minimum touch targets are implemented automatically for Button, EditText/text_input, CheckBox, Switch, Spinner, ListView, and Toolbar. Lowering computes a 48dp pixel floor once per Activity from runtime `densityDpi`, rounds upward, and applies it through `View.setMinimumWidth` and `View.setMinimumHeight`. No IR operation is required. An explicit fixed width below 48dp fails with `AIC1433`; an explicit fixed height below 48dp fails with `AIC1434`. `wrap_content` and `match_parent` remain valid, and non-interactive views may use smaller fixed dimensions.

Strict IR 0.2 accessibility semantics are implemented for non-text views and view state. Every ImageView has exactly one treatment: a typed, non-empty content description when informative or `android.set_decorative(view: ...)` when decorative. ProgressBar requires a content description. Decorative lowering calls `View.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO)` directly. `set_visibility` remains valid for all views, while `set_enabled` accepts only interactive controls; conflicting constant assignments to the same view in one block fail with source-located `AIC1435` through `AIC1441` diagnostics. IR 0.1 compatibility is unchanged.

Literal color properties are complete for IR 0.2. `android.set_text_color` accepts `#RRGGBB` or `#AARRGGBB` literals only for platform TextView subclasses; invalid targets fail with `AIC1442`. `android.set_background_color` accepts every declared view. Both lower through direct platform `Color.parseColor`, `TextView.setTextColor`, and `View.setBackgroundColor` calls, and `ui.color` is advertised only after parser, verifier, optimizer, DEX, fixture, and catalog-parity coverage. IR 0.1 compatibility is unchanged.

Remaining work, in order:

1. Add parser/schema coverage, verifier diagnostics, lowering, capability entries, invalid fixtures, deterministic O0/O1 tests, and runnable examples for every remaining widget/property family.
2. Extend the completed API 36 ARM64 single-device checks across the remaining TalkBack, density, screen-size, and supported-device matrix.

Completion gate: every declared widget/property family has parser, verifier, lowering, invalid fixtures, capability entries, and runnable examples.

### M9.4 — Resources, localization, and project assets — Partial

Implemented: `.aicproject` format 2 stores deterministically ordered, checksummed, bounded PNG/WebP assets and continues reading source-only format 1 archives.

Open: canonical default and BCP-47 string declarations, colors/themes/icons, generated resource IDs, deterministic `resources.arsc` and `res/**` packaging, locale configurations, project-image references, and host-to-compiler asset transfer. Malformed, oversized, duplicate, unsupported, and path-traversing assets must fail before build.

Completion gate: resource output cross-checks against AAPT2, survives locale changes, and produces reproducible APK entries at O0/O1.

### M9.5 — Adaptive layouts, lifecycle, and accessibility corpus — Not started

Add portrait/landscape and compact/expanded variants selected through bounded Android window metrics, then validate rotation, process death, locale changes, font scaling, TalkBack semantics, touch targets, density, and screen size without unrestricted framework calls.

Completion gate: all declared configurations and accessibility scenarios pass on the supported matrix.

### M9.6 — Host, AI, diagnostics, and reference corpus — Partial

Implemented: IR 0.2 AI contract/templates, version display, capability-aware planning inputs, project format 2 codec, initial M9 fixture, ADR, documentation, and build script.

Open: complete structured missing-capability and migration metadata across JNI/host responses, final repair prompts, resource-aware import/export/build flow, stable diagnostics for every new family, and three reference apps covering adaptive navigation, localized image/list UI, and dialog/form restoration.

Completion gate: failed compile, migration, or repair preserves saved projects, while every supported and unsupported corpus request is explained consistently by compiler, host, AI, docs, and runner.

### M9.7 — Acceptance and closure — Blocked

Run workspace tests, Clippy, host JVM/instrumentation tests, deterministic O0/O1 builds, AAPT2/resource and independent DEX inspection, malformed-input tests, accessibility scenarios, and the API 23–36 generated-app/API 30–36 ARM64 host device matrix from one recorded source and toolchain fingerprint.

M9 cannot close until M9.1–M9.6 pass and the pre-existing M8 compatibility matrix closes. Missing devices or partial desktop evidence never count as a pass.

## Resume instruction for a new work session

Read this document, `m9-acceptance-evidence.md`, the IR 0.2 capability catalog, and the current worktree before editing. Preserve all existing uncommitted M9 changes and keep planned catalog entries unsupported until parser, verifier, lowering, fixtures, and tests exist.

Resume M9.3 with **the remaining widget/property families and broader accessibility/device evidence**. Literal text/background colors now have schema, verifier, optimizer, direct DEX lowering, catalog, invalid-fixture, runnable-example, deterministic O0/O1 local coverage, and passing API 36 ARM64 O0/O1 device evidence. Strict informative/decorative image and progress semantics plus visibility/enabled target and conflict validation retain their completed local and API 36 ARM64 O0/O1 evidence. Explicit labels, headings, touch targets, scale-safe text sizing, collections, and selection events retain their recorded device coverage. The broader accessibility and supported-device matrix remains open.

After collections/events, continue through the numbered M9.3 list above. M9.2 typed results and Bundle restoration remain open and must still be completed before M9 closure.
