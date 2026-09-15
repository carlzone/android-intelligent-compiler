# AndroidIntelligentCompiler Milestone Plan

Status: Planning baseline  
Source: `AndroidIntelligentCompiler_Technical_Blueprint.md`  
Project: AndroidIntelligentCompiler (AIC)

## 1. Purpose

This document converts the technical blueprint into an execution and review plan. Each milestone has a fixed scope, concrete deliverables, acceptance criteria, dependencies, and an exit gate. A milestone is complete only when its exit gate is supported by reproducible evidence.

## 2. Program Objective

Build a deterministic compiler pipeline that transforms validated AIC IR into a minimal, signed Android APK, then progressively add programming features, Android capabilities, whole-program optimization, on-device compilation, and AI-assisted intent translation.

```text
Human intent -> AI semantic plan -> deterministic AIC IR
             -> verified compiler pipeline -> DEX/APK -> Android device
```

AI output is always untrusted input. It may propose AIC IR, but parsing, validation, capability checks, lowering, packaging, and signing remain deterministic.

## 3. Milestone Sequence

| Milestone | Outcome | Depends on | Completion evidence |
| --- | --- | --- | --- |
| M0 | Deterministic DEX kernel | Repository bootstrap | Verified, reproducible `classes.dex` |
| M1 | Installable Hello World APK | M0 | Signed APK installed and launched on a physical device |
| M2 | Small typed programming core | M1 | Non-trivial computations execute correctly |
| M3 | Interactive Android UI primitives | M2 | Generated counter and calculator apps work |
| M4 | Persistence and platform capabilities | M3 | Generated offline CRUD app persists data |
| M5 | Whole-program optimization | M4 | Measured improvement with identical behavior |
| M6 | Android host application | M1-M5 compiler stability | Build-to-install flow runs on-device |
| M7 | AI integration | M2 compiler contract; preferably M6 | Prompt-to-APK and prompt-based edits succeed |
| M8 | Toolchain independence | M6 | Supported APK pipeline is self-contained |
| M9 | Production UI and navigation profile | M8 | Adaptive multi-screen application corpus passes |
| M10 | Platform services, data, and runtime | M9 | Connected/offline application corpus passes |
| M11 | AI-native modular development and AIC Studio | M9-M10 language and service contracts | Modular guided/autonomous reference corpus passes |
| M12 | Specialized local AI with Soup | Stable M11 authoring and evaluation contracts | Held-out AIC model evaluation passes |
| M13 | Production hardening, backends, and ecosystem | M8-M12 | Reproducible distribution-ready supported subset |

The sequence is intentionally gated. Work from a later milestone may be researched early, but it must not expand the implementation scope of the active milestone.

## 4. Program-Wide Definition of Done

Every completed milestone must satisfy the following conditions:

1. All milestone acceptance criteria pass in a clean build environment.
1. Automated tests cover new IR, verifier, lowering, and binary-format behavior as applicable.
1. Generated artifacts are checked by an independent Android or DEX verification path where possible.
1. The same compiler version, IR, and API profile produce reproducible output, except for explicitly documented signing or timestamp fields.
1. Compiler errors use typed failures and stable diagnostics; invalid compiler input must not cause a panic.
1. Supported and unsupported behavior is documented.
1. Security and capability implications are reviewed.
1. Benchmark claims include recorded baselines and repeatable measurement steps.
1. No unresolved critical defect remains in the milestone's supported feature set.

## 5. Milestone Details

### M0 - DEX Kernel Proof

Objective: Prove that the Rust compiler can generate a structurally valid and deterministic DEX file containing a minimal class and method graph.

Scope:

- Bootstrap the Rust workspace and the `aic-ir`, `aic-dex`, and `aic-cli` crates.
- Implement checked byte writing, alignment, and endian helpers.
- Implement ULEB128, SLEB128, and the required MUTF-8 subset.
- Model the required DEX header, string, type, prototype, method, class, code, and map sections.
- Centralize sorting, deduplication, indexing, layout, offsets, and fixups.
- Calculate the Adler-32 checksum and SHA-1 DEX signature after final layout.
- Add a CLI command that emits `testdata/generated/minimal.dex`.
- Document the supported DEX subset.

Acceptance criteria:

1. `cargo test` and `cargo clippy` complete without errors.
1. Unit tests cover LEB128, MUTF-8, table ordering/deduplication, alignment, and integrity fields.
1. Layout-derived offsets are used; section offsets are not scattered as hard-coded constants.
1. The minimal DEX passes the selected independent structural verification path.
1. Repeated builds from identical inputs produce byte-for-byte identical output.
1. Invalid compiler inputs return typed errors rather than panicking.

Out of scope: APK packaging, Android host UI, AI integration, a general-purpose language frontend, and Java/Kotlin/Gradle compilation.

Exit gate: A verified minimal DEX can be regenerated byte-for-byte from a clean checkout.

### M1 - First Installable Hello World APK

Objective: Generate, sign, install, and launch an Android APK without Java/Kotlin source or Gradle in the generated-app path.

Scope:

- Define a tiny AIC IR fixture for a Hello World application.
- Lower the IR into a launchable Activity implementation using classic Android View APIs.
- Create the minimal manifest and package metadata for the selected prototype API profile.
- Assemble and align the APK.
- Use official Android tooling as bootstrap/reference tooling for resource packaging and signing.
- Add install and launch scripts for the agreed physical ARM64 test device.
- Record each invoked tool to prove the generated-app path does not call `javac`, `kotlinc`, or Gradle.

Acceptance criteria:

1. The compiler input is AIC IR, not Java or Kotlin source.
1. The generated `classes.dex` contains the required launchable Activity class graph.
1. The manifest declares the package, application, activity, and launcher intent filter.
1. Standard Android tools verify the signed APK.
1. The APK installs on the selected physical device.
1. Launching it displays `Hello from AndroidIntelligentCompiler` without crashing.
1. A clean build log proves the forbidden source compilers and Gradle were not invoked.

Exit gate: A clean, documented run converts the fixture from AIC IR to a signed APK and launches it successfully on the reference device.

### M2 - Small Programming Core

Status: **Complete (2026-09-06)**. Closure evidence is recorded in `compiler/docs/m2-acceptance-evidence.md`.

Objective: Establish a typed semantic core large enough to compile non-trivial deterministic computations.

Scope:

- Add boolean, integer, and string types; document the initial float/decimal strategy.
- Add local variables, mutable and immutable state, functions, parameters, and return values.
- Add arithmetic, comparison, and logical operations.
- Add `if`/`else` and bounded or structured loops.
- Attach source/IR locations to diagnostics.
- Verify types, symbols, and control flow before DEX emission.
- Introduce optimized-versus-unoptimized differential tests.

Acceptance criteria:

1. Typed IR supports integer, boolean, and string variables and functions.
1. Invalid types, undefined symbols, and invalid control flow are rejected before DEX encoding.
1. Diagnostic codes and messages are stable and include useful IR locations.
1. Tests exercise conditional and loop lowering.
1. Non-trivial computation fixtures execute correctly in generated APKs.
1. Differential tests confirm that enabled optimizations preserve observable behavior.

Exit gate: The supported programming core passes semantic, lowering, and device/runtime tests for both valid and invalid fixtures.

### M3 - Interactive Android UI Primitives

Status: **Complete (2026-09-06)**. Closure evidence is recorded in `compiler/docs/m3-acceptance-evidence.md`.

Objective: Generate small interactive Android applications with deterministic UI and event behavior.

Scope:

- Support the `onCreate` lifecycle path.
- Add `TextView`, `Button`, `EditText`, `LinearLayout`, `ScrollView`, and basic layout parameters.
- Lower click listeners and event handlers.
- Reflect application state changes in UI properties.
- Add minimal themes and colors through resources or programmatic configuration.
- Provide counter and calculator fixtures.

Acceptance criteria:

1. Supported UI nodes and properties are validated in IR before lowering.
1. Event handlers update application state and visible UI deterministically.
1. Generated counter and calculator APKs install, launch, and complete their core interaction scenarios.
1. Rotation/recreation behavior is either tested and supported or explicitly documented as unsupported.
1. Unsupported widgets and attributes produce clear diagnostics.

Exit gate: Counter and calculator applications pass repeatable device smoke tests without crashes.

### M4 - Persistence and Platform Capabilities

Status: **Complete (2026-09-06)**. Closure evidence is recorded in `compiler/docs/m4-acceptance-evidence.md`.

Objective: Generate a useful offline CRUD application with persistent state and explicitly governed Android capabilities.

Scope:

- Add a minimal platform-backed key/value persistence path.
- Add SQLite access through Android platform APIs with generated schema and query glue.
- Add explicit IR capability declarations and permission checking.
- Add activity navigation only when required by the reference application.
- Introduce additional system APIs one at a time, each behind a capability boundary and dedicated tests.

Acceptance criteria:

1. A generated CRUD application can create, read, update, and delete records.
1. Data persists across process death and application relaunch.
1. Required capabilities and manifest permissions are derived from verified IR.
1. Undeclared or unsupported capability use is rejected before APK generation.
1. Schema and query failures produce actionable diagnostics.
1. The generated app requests no permissions beyond those required by its verified features.

Exit gate: The reference offline CRUD app passes persistence, relaunch, and permission tests on the reference device.

### M5 - Whole-Program Optimization

Status: **Complete (2026-09-06)**. Closure evidence is recorded in `compiler/docs/m5-acceptance-evidence.md`.

Objective: Reduce generated application cost without changing observable behavior.

Scope:

- Build call, capability, and resource reachability graphs.
- Remove unreachable code and resources.
- Add constant folding and dead-branch elimination.
- Deduplicate strings and resources.
- Add function inlining or specialization only when supported by measurements and tests.
- Establish APK size, cold-start, allocation, memory, and runtime benchmarks.

Acceptance criteria:

1. Every optimization has focused unit tests and optimized-versus-unoptimized differential tests.
1. Benchmark methodology, device profile, inputs, and raw results are recorded.
1. At least one representative generated app shows a measurable improvement in a declared target metric.
1. No supported fixture changes observable behavior.
1. No optimization is retained solely on an unmeasured size or performance claim.

Exit gate: A repeatable benchmark report demonstrates improvement over unoptimized AIC output with equivalent behavior.

### M6 - Android Host Application

Status: **Complete (2026-09-07)**. Closure evidence is recorded in `compiler/docs/m6-acceptance-evidence.md`.

Objective: Run the compiler, package builder, and user-approved installation workflow on an Android device.

Scope:

- Embed the Rust compiler as an ARM64 native library.
- Build a Kotlin host UI for project selection, IR/source editing, building, and diagnostics.
- Store projects in app-managed storage with explicit import/export.
- Integrate the Android `PackageInstaller` user-approval flow.
- Launch generated applications and collect diagnostics available within platform limits.

Acceptance criteria:

1. The host can create or import a project and invoke the embedded compiler without a desktop computer.
1. Build progress, warnings, and errors are visible and actionable.
1. Generated APK installation uses the platform-approved user flow.
1. A generated app can be launched after installation.
1. Project export/import preserves all canonical project data.
1. The compiler core remains independently testable outside the Kotlin host.

Exit gate: The full compile, package, install, and launch workflow succeeds on the reference Android device.

### M7 - AI Integration

Status: **Complete (2026-09-07)**. On the ARM64 reference device, Ollama produced a counter through the reviewed prompt-to-APK flow and completed representative title, control, persistence, and layout edits under deterministic compiler authority; see `compiler/docs/m7-acceptance-evidence.md`.

Objective: Translate natural-language intent into valid AIC IR and support constrained iterative modifications.

Scope:

- Define a provider-independent model interface.
- Publish a formal, versioned IR schema for constrained generation.
- Implement the AI-to-validator repair loop.
- Add patch-oriented edits such as adding controls, persistence, or renaming labels.
- Convert compiler diagnostics into structured repair context.
- Record provenance for model request, response, schema version, and resulting IR changes without exposing secrets.

Acceptance criteria:

1. Model output cannot bypass parsing, schema validation, semantic validation, capability checks, or deterministic lowering.
1. A supported prompt produces valid IR and a working installed APK.
1. Representative patch prompts modify only intended semantic areas and remain reviewable.
1. Invented APIs and undeclared capabilities are rejected with repairable feedback.
1. Failed repair loops terminate with a clear diagnostic and preserve the last valid project state.
1. Signing keys and final signed APK bytes are never directly controlled by the model.

Exit gate: Prompt-to-APK and iterative modification scenarios pass an agreed evaluation suite while all output remains subject to deterministic compiler authority.

### M8 - Toolchain Independence

Status: **Implementation available; acceptance in progress (2026-09-07)**. The binary manifest/direct APK path and host bootstrap removal are implemented. Full API 23-36 generated-app and ARM64 API 30-36 host runtime coverage remains an open exit gate; see `compiler/docs/m8-acceptance-evidence.md`.

Objective: Make the supported on-device production pipeline self-contained within AIC.

Scope:

- Replace the bootstrap AAPT2 dependency for the supported resource subset.
- Replace external alignment and signing steps with verified in-process implementations where justified.
- Ensure the Android host does not download executable compiler tools.
- Maintain a compatibility corpus across supported Android versions and API profiles.
- Cross-check custom outputs against official tools and real devices.

Acceptance criteria:

1. All supported resource, manifest, alignment, and signing paths execute in-process on Android.
1. Custom binary XML/resource output passes independent verification and device tests.
1. Custom signing output verifies using standard Android tooling.
1. The host completes supported builds without downloading executable toolchain components.
1. Compatibility tests pass for every declared Android/API profile.

Exit gate: The supported AIC feature subset compiles, packages, signs, installs, and launches entirely through the self-contained on-device pipeline.

### M9 - Production UI and Navigation Profile

Status: **Implementation in progress (2026-09-08)**. IR 0.2 migration, the capability catalog, multi-Activity packaging,
direct start/finish navigation, and the first layout/accessibility properties are implemented. The remaining corpus and
device exit gates are tracked in `compiler/docs/m9-acceptance-evidence.md`.

M9 execution is divided into durable checkpoints. `Implemented` means the code and focused local tests exist; it does not close the milestone or waive cross-device evidence.

| Phase | Deliverable | Status | Resume condition |
| --- | --- | --- | --- |
| M9.1 | IR 0.2, deterministic 0.1 migration, capability catalog, CLI/JNI/host version contract | Implemented | Keep regression coverage green |
| M9.2 | Multi-screen lifecycle and typed navigation | Partial | Typed extras implemented; start-for-result, typed results, and Bundle restoration remain |
| M9.3 | Production layouts, widgets, reusable rows, and accessibility validation | Implemented | Representative API 36 hardware/API 35 automated matrix complete; non-hardware manual TalkBack explicitly waived |
| M9.4 | Resource table, localization, themes, icons, and project image packaging | Partial | Project archive validation exists; compiler packaging remains |
| M9.5 | Orientation, compact/expanded adaptation, lifecycle and accessibility corpus | Not started | Begin after navigation/state and resources stabilize |
| M9.6 | Host/AI integration, diagnostics, reference applications, and documentation | Partial | Complete against the final M9 language surface |
| M9.7 | Desktop, independent-tool, accessibility, and device acceptance | Blocked by implementation and M8 matrix | Run only from one recorded source/toolchain fingerprint |

Current resume point: **M9.4 canonical resources and deterministic APK resource packaging**. M9.3 is implemented: API 36 ARM64/480dpi hardware passed the full O0/O1 and TalkBack scenarios, while API 35 x86_64 tablet 2560x1600/320dpi and compact 720x1280/240dpi profiles passed O0/O1 automation. The user explicitly waived manual TalkBack on non-hardware profiles; that limitation remains recorded. Exhaustive API compatibility remains in M8/M9.7. M9.2 typed results and lifecycle-safe Bundle restoration remain open before milestone closure.

Objective: Expand AIC from the initial linear-layout profile into a practical, versioned Android UI and navigation language.

Scope:

- Publish a machine-readable capability catalog consumed by the compiler, host, AI prompt layer, documentation, and evaluation tools.
- Version the expanded IR and provide deterministic migration from AIC IR 0.1 projects.
- Add nested layout composition with gravity/alignment, padding, margins, visibility, enabled state, stable dimensions, and adaptive sizing.
- Add production UI primitives in bounded groups: images/icons, lists with reusable rows, menus, dialogs, progress indicators, and common input controls.
- Add multiple screens and explicit navigation, back behavior, activity finish, lifecycle-safe state restoration, and result passing.
- Add resources, themes, localization, accessibility semantics, density handling, and orientation/window-size adaptation.
- Keep every framework operation behind typed verifier and lowering contracts; do not expose arbitrary reflection or unrestricted method invocation.

Acceptance criteria:

1. Every new UI/lifecycle operation has syntax/schema, type and capability validation, deterministic DEX lowering, invalid fixtures, and a runnable device example.
1. The capability catalog can explain before generation whether a requested UI or lifecycle behavior is supported and which capability is missing.
1. Migrated IR 0.1 projects produce behavior equivalent to their original builds.
1. Reference applications cover toolbar actions, multi-screen navigation, lists, dialogs, images, accessibility, localization, and state restoration.
1. Layout tests pass across declared screen sizes, densities, orientation changes, font scaling, and supported Android versions.
1. Unsupported layout or lifecycle requests fail with actionable diagnostics and leave the project unchanged.

Exit gate: AIC can build and run the declared production UI/navigation corpus without Java/Kotlin generated-app compilation, with accessibility and compatibility evidence.

### M10 - Platform Services, Data, and Application Runtime

Objective: Add the platform capabilities required by useful connected and background-aware Android applications.

Scope:

- Add typed HTTP/TLS operations, structured request/response models, timeouts, cancellation, connectivity errors, and explicit network capability declarations.
- Add asynchronous tasks with lifecycle-aware result delivery and deterministic restrictions on concurrency and shared state.
- Expand persistence with transactions, parameterized queries, schema versions, migrations, indexes, and bounded result iteration.
- Add document/media selection through platform contracts and scoped-storage-safe file operations.
- Add runtime permission declaration/request/result flows for a reviewed subset of Android capabilities.
- Add notifications and narrowly scoped scheduled/background work using platform primitives.
- Define privacy, cleartext-network, exported-component, data-retention, and capability-minimization policies enforced before packaging.

Acceptance criteria:

1. Network, asynchronous, database, file, permission, notification, and background-work operations have typed contracts and deterministic lowering tests.
1. Cancellation, recreation, offline behavior, denied permissions, malformed responses, storage exhaustion, and migration failures are covered on device.
1. Generated manifests contain only permissions and components derived from verified capabilities.
1. TLS is the default; cleartext and externally exposed components require explicit policy-approved declarations.
1. Reference applications demonstrate connected data, offline persistence, background scheduling, and recovery without lifecycle leaks or crashes.
1. Static policy checks reject unsafe or undeclared platform access before DEX generation.

Exit gate: The declared connected-application corpus passes functional, lifecycle, privacy, permission, offline, and recovery tests on every supported device profile.

### M11 - AI-Native Modular Development and AIC Studio

Objective: Enable models across a broad capability range to build maintainable applications through modular context, bounded changes, compiler-guided repair, and a modern reviewable host workspace.

The architecture and authoring rules are specified in `compiler/docs/m11-ai-native-modular-studio.md` and `compiler/docs/aic-ai-authoring-standard-0.3.md`.

Scope:

- Publish AIC IR 0.3 and `.aicproject` format 3 with a root manifest and canonical screen, component, domain, service, and resource modules.
- Provide deterministic, reviewable migration from single-file IR 0.2/project format 2; formats 1-2 remain readable and stored projects are never rewritten without approval.
- Add explicit imports/exports, composition-oriented components with typed parameters and events, private state, and bounded service contracts; prohibit wildcard imports, inheritance, reflection, ambiguous symbols, path traversal, and dependency cycles.
- Generate a semantic project index containing compact symbol interfaces, dependencies, navigation, resources, and component usage so prompts receive relevant context rather than the complete project.
- Replace complete-project AI rewriting with atomic file- and symbol-level patch operations protected by project/module hashes and all-or-nothing validation.
- Provide Guided, Assisted, and Autonomous workflows. Recommend page- or feature-level instructions from measured model performance rather than hard-coded model names.
- Ship a versioned standard component set with compiler-enforced behavior, accessibility, and minimum touch-target contracts; keep semantic reuse decisions under AI proposal and user review.
- Redesign the host as the core AIC Studio workspace: AI/blank/template/import creation, project tree, per-module editor, contextual AI panel, application-plan review, per-file diffs, and consolidated build/diagnostic status with responsive phone/tablet navigation.

Acceptance criteria:

1. Large modular multi-screen projects compile deterministically, and physical module or archive ordering cannot change generated output.
1. Invalid paths/imports, cycles, duplicate or ambiguous symbols, stale hashes, and component contract violations produce stable file-and-source-located diagnostics.
1. AI proposals touch only declared files/symbols; validation is transactional and every failure preserves the complete prior project.
1. Context retrieval omits unrelated implementations while supplying every required interface and capability constraint.
1. A qualified lower-capability model completes the reference application through Guided page-by-page work, and a qualified higher-capability model completes it from a whole-app description through an approved plan and bounded Autonomous patches.
1. Both workflows pass the same compiler, accessibility, compatibility, and device gates; reports include first-pass compilation, repair success, unintended changes, unsupported-capability accuracy, token use, and review steps.
1. AIC Studio completes create/import, browse, edit, plan/review/apply, build, install, and interruption-recovery scenarios on supported phone and tablet layouts.

Deferred: synchronized visual source editing, navigation-graph editing, multi-configuration device previews, and component galleries are post-M11 enhancements.

Exit gate: A modular reference corpus can be authored by both guided lower-capability and autonomous higher-capability model workflows, reviewed in AIC Studio, and built through the same deterministic production gates.

### M12 - Specialized Local AI with Soup

Objective: Train and ship an optional local model specialized for AIC planning, constrained edits, repair, and honest unsupported-intent reporting.

Scope:

- Extend the provider-neutral proposal contract with explicit `create`, `patch`, and `unsupported` outcomes plus referenced capability IDs.
- Prefer structured semantic edit operations over complete-source rewriting where the compiler owns an equivalent deterministic transformation.
- Build a versioned dataset from reviewed valid programs, minimal edits, compiler repair traces, unsupported requests, and adversarial negative examples; remove secrets and user content not approved for training.
- Establish train/validation/held-out prompt splits that prevent template and project leakage.
- Use a pinned Soup release to run reproducible QLoRA/SFT experiments on a suitable Qwen instruct checkpoint; record base model, tokenizer/chat template, dataset digest, seed, adapter configuration, and hardware.
- Evaluate retrieval-only, constrained-decoding, base-model, and fine-tuned variants before selecting a model.
- Merge the accepted adapter, export a quantized GGUF, and serve it locally through llama.cpp and/or Ollama using the existing provider abstraction.
- Keep compiler validation, capability enforcement, signing, packaging, installation, and launch outside model authority.

Acceptance criteria:

1. Dataset licenses, provenance, redaction, splits, versioning, and reproducible generation are documented and audited.
1. Held-out evaluation measures schema validity, compile success, semantic task success, minimal-diff behavior, repair convergence, capability citation, and unsupported-intent accuracy.
1. The specialized model materially improves over the selected base model without regressing safety or unsupported-request handling.
1. Model quantization preserves the agreed evaluation thresholds and fits the documented local hardware envelope.
1. Exported GGUF artifacts run through both the Android host provider boundary and at least one supported local server.
1. Model, adapter, dataset, and evaluation versions are independently selectable; the compiler remains fully usable without them.

Exit gate: A pinned Soup-trained local model passes the held-out AIC evaluation suite and completes the representative prompt-to-APK corpus while correctly declining unsupported requests.

### M13 - Production Hardening, Backends, and Ecosystem

Objective: Mature the expanded supported profile into a reproducible, governed, distribution-ready toolchain.

Scope:

- Evaluate an optional LLVM/ARM64 backend for measured compute-heavy hotspots.
- Add AAB/release support only after the APK pipeline and expanded Android profile are stable.
- Establish a versioned catalog of reusable trusted primitives and capability modules.
- Finalize IR migration/compatibility policy across all published versions.
- Define plugin/capability governance, dependency review, supply-chain metadata, and reproducible release builds.
- Harden signing/key management and complete publishing/distribution requirements.
- Run security review, fuzzing, performance, power, accessibility, compatibility, upgrade, rollback, and long-duration reliability gates.

Acceptance criteria:

1. Optional backend selection is based on repeatable profiling and produces equivalent results.
1. Published IR versions have documented compatibility and tested migration paths.
1. Trusted primitives and capability modules have explicit ownership, security boundaries, and versioning.
1. Release artifacts are reproducible to the documented limit and include toolchain, capability, model, and dataset metadata where applicable.
1. Signing keys are handled through an audited design appropriate to the selected distribution model.
1. The supported production subset, limitations, compatibility matrix, support policy, and update strategy are published.

Exit gate: A declared production-grade subset builds reproducibly and passes compatibility, security, performance, accessibility, reliability, and distribution-readiness reviews.

## 6. Cross-Milestone Test Growth

| Test layer | Introduced | Continues through |
| --- | --- | --- |
| Pure unit tests for encoding and lowering | M0 | M13 |
| Golden DEX/APK fixtures | M0 | M13 |
| Independent structural verification | M0 | M13 |
| Physical-device install/launch smoke tests | M1 | M13 |
| Semantic valid/invalid IR fixtures | M2 | M13 |
| Optimization differential tests | M2 | M13 |
| Interactive UI tests | M3 | M13 |
| Persistence and permission tests | M4 | M13 |
| Size and performance benchmarks | M5 | M13 |
| On-device workflow tests | M6 | M13 |
| AI evaluation and repair-loop tests | M7 | M13 |
| Android/API compatibility corpus | M8 | M13 |
| Adaptive UI, navigation, accessibility, and migration tests | M9 | M13 |
| Network, async, permission, privacy, and recovery tests | M10 | M13 |
| Modular project, targeted-patch, context-retrieval, and Studio workflow tests | M11 | M13 |
| Model dataset, calibration, quantization, and held-out evaluations | M12 | M13 |

## 7. Dependencies and Decision Gates

The following decisions must be made before the indicated work begins:

| Decision | Needed by | Required evidence or owner action |
| --- | --- | --- |
| Prototype `minSdk`, `targetSdk`, and API profile | Before M1 implementation | Record in an ADR and compiler profile |
| Physical ARM64 reference device and Android version | Before M1 device acceptance | Record device/build identifiers in test evidence |
| Independent DEX verification tool/path | Before M0 exit | Document version and command |
| Debug signing-key handling | Before M1 packaging | Document storage and regeneration policy |
| Initial float/decimal semantics | During M2 | Record type and lowering decision in an ADR |
| First persistence backend | Before M4 | Select platform primitive and document supported semantics |
| AI provider/model | Before M7 | Select behind provider abstraction; do not embed into compiler core |
| Supported Android/API compatibility matrix | Before M8 exit | Publish tested profiles and exclusions |
| Expanded IR version and UI capability boundary | Before M9 | Approve capability catalog and migration ADR |
| Platform-service and permission subset | Before M10 | Approve threat model, privacy policy, and Android API list |
| IR 0.3 module, patch, migration, and Studio interaction contracts | Before M11 | Approve modular-project and AI-authoring ADRs |
| Soup/base model, dataset license, and training hardware | Before M12 | Record reproducible training and evaluation ADR |
| Release distribution and key-management model | Before M13 exit | Complete security and policy review |

## 8. Milestone Tracking Template

Copy this section for each milestone when implementation starts.

```text
Milestone:
Owner:
Status: Not started | In progress | Blocked | In review | Complete
Start date:
Target review date:

Scope committed:
-

Acceptance evidence:
- Test report:
- Generated artifacts:
- Independent verification:
- Device/build profile:
- Benchmark report (if applicable):
- Documentation/ADR updates:

Open blockers:
-

Deferred items:
-

Exit-gate decision:
- Decision: Pass | Conditional pass | Fail
- Reviewers:
- Date:
- Notes:
```

## 9. Immediate Execution Plan

1. Bootstrap the proposed repository structure.
1. Write the architecture decision record locking Rust, custom AIC IR, and direct DEX as the prototype architecture.
1. Select the prototype Android API profile, physical ARM64 test device, and independent DEX verification path.
1. Start M0 only and produce the deterministic DEX kernel evidence.
1. Begin M1 only after the M0 exit gate passes.
1. Do not start AI integration until the deterministic compiler contract is reliable; M1 and M2 are the minimum foundation.

## 10. Program Completion Statement

AndroidIntelligentCompiler reaches its planned end state when a governed, production-grade supported subset can reproducibly transform intent into validated AIC IR and then into an optimized Android artifact through a deterministic, self-contained toolchain, with real-device compatibility and security evidence.
