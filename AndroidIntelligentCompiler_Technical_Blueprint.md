**AndroidIntelligentCompiler**

Technical Blueprint, Requirements & Phased Implementation Plan

Codename / repository name: AndroidIntelligentCompiler  
Status: Concept approved for technical prototyping  
Document date: 6 September 2026

**Core thesis**

*Human intent -> AI semantic plan -> deterministic compiler IR -> optimized DEX/APK -> Android device*

# 1. Executive Summary

AndroidIntelligentCompiler (AIC) is an experimental Android-native application compiler designed around a different assumption from conventional development tools: source code no longer has to be optimized primarily for human authorship. An AI system can operate at the intent and semantic layers, while a deterministic compiler produces the exact executable representation required by Android.

The long-term goal is not to place Android Studio inside a phone. The goal is to build a compact compiler/runtime toolchain that can synthesize legitimate standalone Android APKs directly on an Android device, with minimal generated code, minimal dependencies, and aggressive whole-program specialization.

The first implementation deliberately excludes AI. The initial proof must show that a custom compiler can produce a valid installable Android application from a small custom intermediate representation (IR), without compiling Java or Kotlin source and without using Gradle for the generated application.

## Project-name assessment

AndroidIntelligentCompiler is a strong engineering/project name. It communicates Android as the target, intelligence/AI as the planning and optimization layer, and compiler construction as the core technical problem. The consumer-facing app name can remain undecided until the product experience becomes clearer.

## Definition of success

```
Input:   A small application specification / AIC IR
Compiler: deterministic Rust compiler
Output:  signed APK containing valid Android package metadata + classes.dex
Device:  Android phone installs and runs the APK

Generated app compilation path must use:
  NO Java source
  NO Kotlin source
  NO javac / kotlinc
  NO Gradle
```

# 2. Product Vision and Design Principles

- Intent-first: users describe behavior; the AI translates intent into a constrained semantic representation.
- Deterministic compilation: AI does not directly emit trusted APK binary structures. The compiler validates and lowers IR deterministically.
- Android-first: target one operating system first, exploiting Android framework APIs and ART/DEX directly.
- Minimal by construction: include only reachable app logic, resources, and generated helper code.
- Platform reuse: prefer Android platform capabilities over shipping custom copies of functionality already present on the OS.
- Generated code is disposable: the canonical source of truth is semantic intent + IR, not human-friendly boilerplate.
- Security-sensitive primitives remain trusted: cryptography, TLS, authentication, payment, and similarly critical components should rely on verified implementations/platform APIs.
- Build from the smallest proven kernel upward: each phase must produce a runnable result before expanding the language or platform surface.
# 3. Conceptual Architecture

```
┌─────────────────────────────┐
│       User / Developer      │
│   natural-language intent   │
└──────────────┬──────────────┘
               │
               v
┌─────────────────────────────┐
│        AI Planning Layer    │
│ intent -> semantic app plan │
└──────────────┬──────────────┘
               │ validated AIC IR
               v
┌─────────────────────────────┐
│      Deterministic Compiler │
│ parser / verifier / types   │
│ optimizer / lowering        │
├──────────────┬──────────────┤
│ DEX backend  │ APK/resources│
└──────────────┴──────┬───────┘
                      │
                      v
              Signed Android APK
                      │
                      v
               Android Package
               Manager + ART
```

## Important separation of responsibilities

| Layer | Responsible for | Must not be trusted for |
| --- | --- | --- |
| AI | Understanding intent; creating/editing semantic IR; suggesting optimizations; diagnosing failures | Raw DEX offsets/checksums, binary XML correctness, APK signature correctness |
| IR validator | Types, lifecycle legality, capability/permission rules, graph consistency | Guessing missing product behavior |
| Optimizer | Reachability, constants, dead-code elimination, specialization, resource pruning | Changing externally observable semantics |
| Backend | Exact DEX instruction encoding and Android API call lowering | Product-level interpretation |
| Packager/signer | APK layout, binary metadata/resources, alignment/signature | App business logic |

# 4. Technology Decisions

| Area | Initial decision | Reason |
| --- | --- | --- |
| Compiler core | Rust | Systems-level binary control with memory safety; suitable for DEX/APK writers and ARM64 native library builds. |
| Android host app | Kotlin, thin shell | Best fit for Activity/UI, permissions, package installer interaction, storage and Android lifecycle integration. |
| Compiler API boundary | JNI/FFI to Rust native library | Keeps correctness-critical compiler reusable and mostly independent from Android UI. |
| Canonical program representation | Custom AIC IR | Stable AI/compiler contract; avoids tying the project to Kotlin/Java syntax or framework conventions. |
| Primary managed backend | DEX | ART executes DEX; direct DEX generation removes Java/Kotlin compilers from the generated-app pipeline. |
| Native backend | Deferred; LLVM/ARM64 candidate | Useful later for compute-heavy routines, but unnecessary for first proof. |
| Resources/manifest | Bootstrap using AAPT2 if needed; replace progressively | AAPT2 is proven; custom resource/binary-XML implementation is a later independence milestone. |
| APK signing | Bootstrap with apksigner; implement/verify native signing later | Signing correctness is security-critical; avoid blocking the compiler proof on reimplementing it. |
| AI provider | Provider-independent abstraction | Compiler must remain usable/testable without a specific model service. |

## Why not C/C++ as the primary compiler language?

C and C++ are fully capable choices. Rust is preferred because the project manipulates attacker-influenced structured binary data, indexes, offsets, byte buffers, signatures, and serialized graphs. Memory safety reduces risk in exactly these areas while retaining native performance and low-level control.

## Why Kotlin remains in the project

Kotlin is not a generated-app dependency. It is a convenience layer for the Android host application. The compiler itself should remain usable as a Rust library and eventually as a command-line/native test harness independent of the Android UI.

# 5. Android Facts that Constrain the Design

- ART executes the DEX format and DEX bytecode; therefore DEX is a legitimate direct compiler target.
- DEX files have strict structural/integrity constraints, including magic/version, checksums/signatures, offsets, indexes, alignment and encoded tables. The backend must be deterministic and heavily tested.
- AAPT2 compiles and links Android resources into Android-optimized binary formats and can be used from a custom command-line build system; Gradle is not required to invoke it.
- Every installable APK must be digitally signed. Modifying a signed APK invalidates the signature; alignment must occur before signing when that workflow is used.
- Modern Android imposes application sandbox and executable-code restrictions. The compiler should be shipped as a native library inside the host APK rather than depending on downloading arbitrary executable tool binaries into writable app storage.
- The generated app should invoke Android framework functionality that already exists on the device rather than repackaging equivalent general-purpose libraries whenever feasible.
# 6. Required Development Environment

These tools are required to build AIC itself. They are not necessarily dependencies of the applications generated by AIC.

| Requirement | Purpose | Required at project start? |
| --- | --- | --- |
| Git + GitHub/repository hosting | Version control, review, CI | Yes |
| Rust stable toolchain (rustup/cargo) | Compiler core, tests, binary writers | Yes |
| Android Studio / Android SDK | Build and debug the AIC host application | Yes |
| Android NDK | Build/link Rust native library for Android ARM64 | Yes |
| JDK required by Android host build tooling | Build AIC host app during development | Yes; host only |
| Kotlin Android project | Thin AIC host UI/integration | Yes, after desktop compiler kernel starts |
| Physical ARM64 Android device | Real installation/runtime validation | Yes |
| ADB / platform-tools | Install, launch, logcat and automated integration tests | Yes for development workstation |
| AAPT2 | Bootstrap manifest/resource packaging | Optional in earliest desktop tests; likely Phase 0/1 bootstrap |
| apksigner + zipalign/build-tools | Bootstrap APK finalization/signing | Yes for first installable proof unless implemented directly |
| AI API/model integration | Natural language -> IR | No; defer until deterministic compiler is proven |
| LLVM | Future native-code backend | No |

## Recommended initial platform constraints

- Host and compiler: ARM64-v8a only.
- Generated apps: one fixed minSdk/targetSdk profile selected and version-controlled for the prototype.
- Single Activity, no services/providers/receivers initially.
- No third-party runtime libraries in generated apps.
- Classic Android View APIs first; Compose is intentionally excluded from the compiler proof.
- Debug/self-signed generated applications first; release/Play publishing is out of initial scope.

# 7. Proposed Repository Structure

```
AndroidIntelligentCompiler/
├── README.md
├── docs/
│   ├── architecture.md
│   ├── ir-spec.md
│   ├── dex-backend.md
│   ├── apk-format.md
│   └── milestones.md
├── compiler/                 # Rust workspace
│   ├── aic-ir/               # IR model + parser/serializer
│   ├── aic-verify/           # semantic/type verification
│   ├── aic-opt/              # whole-program optimizer
│   ├── aic-dex/              # DEX structures + encoder
│   ├── aic-android/          # lowering Android primitives -> DEX
│   ├── aic-apk/              # APK assembly/metadata abstraction
│   ├── aic-sign/             # later native signing implementation
│   └── aic-cli/              # desktop test harness
├── android-host/             # Kotlin Android app
│   └── app/
├── examples/
│   ├── hello-world.aic
│   ├── counter.aic
│   └── calculator.aic
├── testdata/
│   ├── dex/
│   └── apk/
└── scripts/
    ├── install-example.sh
    └── verify-apk.sh
```

## Why create a desktop CLI first?

The compiler is much easier to debug with deterministic fixture files and unit tests on a workstation. Once the exact same Rust library produces a valid APK there, it can be embedded in the Android host. This prevents Android UI/JNI issues from obscuring compiler-format bugs.

# 8. AIC Intermediate Representation (IR)

The IR is the most important public contract in the architecture. AI outputs IR; humans may inspect it; the deterministic compiler consumes it. It should be typed, versioned, small, explicit and independently serializable.

## Initial characteristics

- No implicit imports or dependency resolution.
- Explicit Android capabilities and permissions.
- Typed values and function signatures.
- Explicit lifecycle entry points such as on_create.
- Structured control flow rather than arbitrary textual snippets.
- Stable IDs for views/state/functions to support AI patching and deterministic diffs.
- IR schema version embedded in every program.
```
aic_version 0.1
app "Hello" package "dev.aic.generated.hello" {
  activity Main {
    on_create {
      let root = android.linear_layout(orientation: vertical)
      let message = android.text_view(text: "Hello from AIC")
      android.add_view(parent: root, child: message)
      android.set_content_view(root)
    }
  }
}
```

## IR evolution strategy

Do not attempt to design a complete programming language at the beginning. Add one semantic capability only when a milestone requires it. Every addition must include: syntax/schema, verifier rules, lowering rules, tests, and at least one runnable example.

# 9. Compiler Pipeline

```
AIC IR text / structured object
        │
        v
[1] Parse / deserialize
        │
        v
[2] Semantic verifier + type checker
        │
        v
[3] High-level normalized IR
        │
        v
[4] Whole-program optimizer
        │
        v
[5] Android lowering
        │
        v
[6] DEX-oriented low-level IR
        │
        v
[7] Register allocation / instruction selection
        │
        v
[8] DEX encoder + integrity fields
        │
        v
[9] Manifest/resources/APK assembly
        │
        v
[10] Align + sign + verify
        │
        v
     installable APK
```

## Minimum optimizer passes

| Pass | When to add | Purpose |
| --- | --- | --- |
| Reachability / dead-function elimination | Early | Remove unreachable generated functions and helpers. |
| Constant folding | Early | Resolve static expressions and simplify branches. |
| Dead branch elimination | Early | Remove unreachable control-flow arms after constants are known. |
| String/resource deduplication | Early-medium | Avoid duplicate constants/assets. |
| Function inlining | Medium | Reduce call overhead and expose more specialization opportunities. |
| Function specialization | Medium | Generate variants for known constant/type arguments and remove generic paths. |
| Escape/allocation analysis | Later | Avoid objects/allocations when scalar/local representation is sufficient. |
| Native-code selection | Much later | Move compute-heavy kernels to a native backend when objectively beneficial. |

# 10. Phased Implementation Roadmap

## Phase 0 - DEX Kernel Proof

Goal: prove that our own Rust code can produce a structurally valid DEX containing a minimal class/method graph. No AI, no Android host UI.

- Implement byte writer, endian helpers, ULEB128/SLEB128, MUTF-8 encoding.
- Implement minimal DEX tables: strings, types, prototypes, methods, class definitions, code items, map/header.
- Generate checksum/signature fields correctly.
- Create deterministic golden test fixtures and parse/inspect output with an independent verifier/tool where possible.
> Milestone M0: compiler produces a valid classes.dex for a minimal MainActivity-like class graph.

## Phase 1 - First Installable Hello World APK

Goal: install and launch an APK generated without Java/Kotlin source or Gradle for the generated app.

- Lower a tiny AIC IR into DEX calls to Android framework classes.
- Create/compile minimal AndroidManifest package metadata.
- Assemble APK, align, sign and verify.
- Install on a physical Android device and launch via ADB/system launcher.
- Display “Hello from AndroidIntelligentCompiler” using classic Android View APIs.
> Milestone M1 (foundation proof): AIC IR -> Rust compiler -> classes.dex -> signed APK -> installed/running Hello World.

## Phase 2 - Small Programming Core

- Primitive types: bool, integer, float/decimal strategy, string.
- Local variables, immutable/mutable state, functions, parameters and return values.
- Arithmetic/comparison/logical operations.
- if/else and bounded/structured loops.
- Compiler diagnostics with source/IR locations.
- Verifier catches invalid types, undefined symbols and invalid control flow before DEX emission.
> Milestone M2: non-trivial pure computation examples execute correctly inside generated APKs.

## Phase 3 - Interactive Android UI Primitives

- Activity lifecycle: onCreate first; add others only as needed.
- TextView, Button, EditText, LinearLayout, ScrollView and basic layout parameters.
- Event listeners/onClick lowering.
- State updates reflected in UI.
- Basic theme/colors using minimal resources or programmatic configuration.
> Milestone M3: Counter and calculator APKs are generated, installed and function correctly.

## Phase 4 - Persistence and Platform Capabilities

- SharedPreferences or DataStore-like minimal state path (prefer direct platform primitives).
- SQLite through Android platform APIs; compiler-generated schema/query glue.
- Intent-based navigation between activities only if/when required.
- Permissions represented explicitly in IR and checked against used capabilities.
- Selected system APIs: notifications, files, network, camera only one at a time with tests.
> Milestone M4: a useful offline CRUD application can be generated with persistent data.

## Phase 5 - Whole-Program Optimization

- Construct call/capability/resource reachability graph.
- Dead-code and dead-resource removal.
- Specialize generated helper functions for known app constraints.
- Measure APK size, startup, allocations and runtime before/after each optimization.
- Introduce regression benchmarks to ensure “smaller” does not silently become “slower” or incorrect.
> Milestone M5: measurable reduction versus unoptimized AIC output, with identical behavior.

## Phase 6 - Android Host Application

- Embed Rust compiler as native ARM64 library.
- Kotlin UI: project list, IR/source view, build button, build output, APK install action.
- Store projects in app-managed storage with export/import.
- PackageInstaller flow / user-approved APK installation.
- Launch generated app and collect diagnostics available within platform constraints.
> Milestone M6: entire compile -> package -> install workflow executes from the Android device.

## Phase 7 - AI Integration

- Model-provider abstraction.
- Natural language -> AIC IR generation constrained by a formal schema/specification.
- IR validator feedback loop: AI repairs semantic errors, never binary structures.
- Patch-oriented edits: “add a reset button”, “store history”, “rename title”.
- Compiler/runtime diagnostics -> structured context -> AI suggested IR fix.
> Milestone M7: prompt -> valid IR -> compiled APK -> install, plus iterative prompt-based modification.

## Phase 8 - Toolchain Independence

- Replace bootstrap AAPT2 dependency for the supported resource subset with our own binary XML/resource writer.
- Replace external alignment/signing steps with verified in-process implementations where justified.
- Ensure the Android host does not need to download executable compiler tools.
- Keep compatibility test corpus against Android releases/API profiles.
> Milestone M8: generated app production pipeline is self-contained inside AIC for the supported feature subset.

## Phase 9 - Production UI and Navigation Profile

- Publish a machine-readable capability catalog shared by compiler validation, host UX, AI planning, documentation, and evaluation.
- Version and migrate the IR before broadening it beyond the 0.1 linear-layout subset.
- Add typed layout composition, alignment/gravity, spacing, visibility/state, adaptive dimensions, lists, images, dialogs, menus, and common controls.
- Add multiple screens, explicit navigation/back/finish operations, lifecycle-safe restoration, resources, themes, localization, and accessibility semantics.
- Require parser/schema, verifier, lowering, invalid fixtures, device tests, and compatibility evidence for every operation.
> Milestone M9: AIC supports a declared production UI/navigation corpus across the supported device matrix.

## Phase 10 - Platform Services, Data, and Application Runtime

- Typed HTTP/TLS and structured data exchange with cancellation and offline errors.
- Lifecycle-aware asynchronous work with deterministic concurrency restrictions.
- Transactional/versioned persistence and migration, scoped files/document contracts, runtime permissions, notifications, and scheduled work.
- Compiler-enforced privacy, exported-component, cleartext-network, data-retention, and least-capability policies.
> Milestone M10: AIC supports a declared connected-application corpus with permission, privacy, lifecycle, offline, and recovery evidence.

## Phase 11 - Specialized Local AI with Soup

- Add explicit supported/unsupported planning and capability IDs to the provider-neutral model contract.
- Prefer compiler-owned semantic edit operations over unconstrained complete-source rewriting.
- Build licensed, versioned SFT/preference/evaluation data from valid programs, repairs, minimal edits, and unsupported/adversarial requests.
- Use a pinned Soup release for reproducible QLoRA/SFT experiments on a selected Qwen instruct model.
- Compare base, retrieval, constrained-decoding, and fine-tuned systems on held-out schema, compile, behavior, minimal-diff, repair, and refusal metrics.
- Merge and export the accepted model as quantized GGUF for local llama.cpp/Ollama serving while retaining deterministic compiler authority.
> Milestone M11: the specialized local model passes the held-out AIC suite and representative prompt-to-APK scenarios, including honest unsupported responses.

## Phase 12 - Production Hardening, Backends, and Ecosystem

- Optional LLVM/ARM64 backend only for measured compute-heavy hotspots.
- AAB/release pipeline after the APK compiler and expanded application profile are stable.
- Reusable trusted primitive catalog, governed capability modules, IR compatibility, reproducible builds, and supply-chain metadata.
- Publishing/distribution review, hardened signing/key management, fuzzing, security, accessibility, performance, power, upgrade, and reliability gates.
> Milestone M12: AIC supports a governed production-grade subset with reproducible, optimized, distribution-ready builds.

# 11. Detailed First Three Milestones for Codex

The first Codex implementation work should remain narrow. Do not create the Android host UI until M1 works from a desktop/test CLI.

## M0 - DEX Writer Acceptance Criteria

1. A Rust workspace builds with cargo test and cargo clippy without errors.
1. aic-dex can encode a deterministic DEX header and the minimal referenced sections needed by its fixture.
1. Checksum and SHA-1 DEX signature are calculated after final layout.
1. Offsets and required alignment are calculated by a layout pass, not hard-coded ad hoc throughout the writer.
1. Unit tests cover LEB128, MUTF-8, sorted/deduplicated indexes and checksum/signature generation.
1. A golden fixture is reproducible byte-for-byte from identical input.
## M1 - Hello World APK Acceptance Criteria

1. Input is an AIC IR fixture, not Java/Kotlin source.
1. The compiler generates classes.dex that defines/uses a launchable Activity implementation appropriate to the selected prototype API profile.
1. A valid manifest declares package/application/activity and launcher intent filter.
1. APK can be signed and verified using standard Android tooling during bootstrap.
1. APK installs on the agreed physical test device.
1. Launching the app shows a visible Hello World string and does not crash.
1. Build script proves no javac, kotlinc or Gradle invocation is used in the generated-app path.
## M2 - Programming Core Acceptance Criteria

1. Typed IR supports integer/bool/string variables and functions.
1. Type errors are rejected before DEX encoding with stable diagnostic codes/messages.
1. Control-flow tests exercise if/else and loop lowering.
1. Generated behavior is covered by device/instrumented or black-box runtime tests.
1. No optimization pass is permitted to change observable behavior; differential tests compare optimized and unoptimized results.
# 12. Testing Strategy

| Test layer | Examples | Purpose |
| --- | --- | --- |
| Pure unit tests | LEB128, MUTF-8, table sorting, opcode encoding, register ranges | Catch binary-format bugs quickly. |
| Golden binary tests | Known IR -> exact DEX bytes for small fixtures | Reproducibility and regression detection. |
| Structural verification | Independent DEX/APK inspection and Android verifier/install | Avoid validating our output only with our own parser. |
| Semantic compiler tests | Valid/invalid IR fixtures | Verifier/type system behavior. |
| Device smoke tests | Install, launch, interact, check crash status | Confirm real Android compatibility. |
| Optimization differential tests | Optimized vs unoptimized outputs/behavior | Prove transformations preserve semantics. |
| Fuzz/property tests | DEX encoding inputs, malformed IR | Hardening against parser/serializer edge cases. |
| Size/performance benchmarks | APK bytes, cold start, allocations, memory | Quantify whether the project meets its stated optimization thesis. |

# 13. Security and Trust Model

- Treat AI output as untrusted input. It must pass parsing, schema validation, semantic verification, capability checks and deterministic lowering.
- Never give the model raw authority to write signing keys or mutate final APK bytes after signing.
- Generated apps should request only permissions implied by verified IR capabilities.
- Prefer platform cryptography/TLS/security APIs. Do not synthesize novel crypto implementations.
- Keep debug signing keys separate from future release signing infrastructure.
- Make builds reproducible where feasible and record compiler/IR schema/API-profile versions in build metadata.
- Fuzz binary readers/writers and validate size/offset arithmetic to prevent corruption and memory abuse.
# 14. Major Technical Risks

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| DEX verifier complexity | A byte-perfect DEX can still violate verifier/type/register rules. | Start with tiny instruction subset; add independent verification and device tests for each opcode family. |
| Android API evolution | Framework behavior/requirements change across API levels. | Pin one prototype API profile; version lowering rules; expand only with compatibility tests. |
| Resources/binary XML complexity | Full Android resource semantics are large. | Programmatic UI first; bootstrap AAPT2; support a deliberately tiny custom resource subset later. |
| Signing/package correctness | Invalid signatures make APKs uninstallable; key errors are high impact. | Use official signer for bootstrap; add native implementation only with cross-verification. |
| AI semantic hallucination | Model can invent APIs/capabilities. | Closed IR vocabulary, schema-constrained generation, compiler validator as authority. |
| Generated-code security | Specialized code may miss edge cases. | Trusted primitive boundary; static checks; tests; platform APIs for sensitive operations. |
| Size ≠ speed | Tiny code may use poor algorithms or harm runtime performance. | Benchmark multiple objectives; never optimize APK bytes alone. |
| On-device restrictions | Android sandbox restricts arbitrary executable tooling. | Ship compiler as native library; prefer in-process compilation/package generation. |
| Scope explosion | Recreating Android/Java/Kotlin ecosystems is unbounded. | Feature-gated roadmap; milestones define the only supported surface at each phase. |

# 15. Non-Goals for the Initial Project

- Reimplementing Android Studio.
- Supporting arbitrary Java/Kotlin source input.
- Supporting Gradle plugins or Maven dependency graphs.
- Supporting Jetpack Compose in the first compiler generation.
- Compiling C/C++ applications generally.
- Replacing Android platform services such as SQLite, TLS, rendering, camera or notification engines.
- Play Store publishing in the prototype.
- iOS support.
- Multi-architecture support before ARM64 is stable.
# 16. Implementation Rules for Codex

Use these as persistent engineering constraints when handing the repository to Codex or another coding agent.

1. Do not broaden scope beyond the active milestone.
1. Prefer deterministic, testable compiler transformations over model-driven binary generation.
1. Every new IR feature must include parser/schema, verifier, lowering, tests and a runnable example.
1. Never introduce a third-party dependency into generated APKs merely to simplify compiler implementation without documenting the generated size/runtime cost.
1. Keep Android platform calls behind a compiler capability/lowering layer; do not scatter stringly-typed method descriptors throughout the codebase.
1. Centralize DEX indexing, sorting, layout, offsets, alignment and integrity calculations.
1. Keep the Rust compiler independent from the Kotlin Android host wherever possible.
1. Use official Android tooling only as bootstrap/reference/oracle components, with explicit issue labels for dependencies intended to be removed.
1. Measure before claiming an optimization. Record APK size and runtime benchmarks.
1. Preserve reproducibility: same compiler version + same IR + same profile should produce deterministic output except explicitly documented signature/timestamp fields.
# 17. Suggested First Codex Work Package

```
WORK PACKAGE: AIC-M0 DEX Kernel

Objective:
Create the initial Rust workspace and implement the minimal deterministic DEX writing primitives.

Deliverables:
1. compiler/Cargo.toml workspace
2. crates: aic-ir, aic-dex, aic-cli
3. ByteWriter with checked offsets/alignment
4. ULEB128 + SLEB128 encoding and tests
5. MUTF-8 encoder sufficient for initial identifiers/strings + tests
6. DEX model for header, string/type/proto/method/class/code subsets
7. deterministic index builders (sorted + deduplicated where DEX requires it)
8. layout/fixup pass
9. Adler-32 + SHA-1 integrity generation
10. CLI command that writes testdata/generated/minimal.dex
11. golden/reproducibility tests
12. docs/dex-backend.md describing supported subset and unsupported items

Constraints:
- No Android UI work.
- No AI integration.
- No Java/Kotlin/Gradle generated-app compilation.
- No premature general-purpose language frontend.
- Every unsafe Rust block, if any, requires explicit justification.
- Errors must use typed error variants; do not panic on invalid compiler input.

Exit condition:
M0 is complete only when the generated DEX passes the selected independent structural verification path and is byte-for-byte reproducible.
```

# 18. Decision Log - Locked vs Deferred

| Decision | Status |
| --- | --- |
| Project/repository name: AndroidIntelligentCompiler | LOCKED for engineering use |
| Primary target: Android only | LOCKED |
| Compiler core: Rust | LOCKED for prototype |
| Android host shell: Kotlin | LOCKED for prototype |
| Primary executable target: DEX | LOCKED |
| Canonical app representation: custom AIC IR | LOCKED |
| Classic View APIs before Compose | LOCKED |
| AI introduced only after deterministic compiler proof | LOCKED |
| AAPT2/apksigner allowed as bootstrap/reference tools | LOCKED |
| Exact minSdk/targetSdk/API profile | DEFERRED to repository bootstrap decision |
| Consumer-facing app name | DEFERRED |
| AI provider/model | DEFERRED |
| Native LLVM backend | DEFERRED |
| AAB / Play publishing | DEFERRED |
| Custom resource compiler/signing implementation details | DEFERRED until APK proof works |

# 19. Reference Sources

Authoritative references checked for this blueprint (accessed September 2026):

- **Android Open Source Project - Dalvik executable format: **https://source.android.com/docs/core/runtime/dex-format
- **Android Open Source Project - DEX constraints: **https://source.android.com/docs/core/runtime/constraints
- **Android Open Source Project - Dalvik executable instruction formats: **https://source.android.com/docs/core/runtime/instruction-formats
- **Android Open Source Project - Android runtime and Dalvik: **https://source.android.com/docs/core/runtime
- **Android Developers - AAPT2: **https://developer.android.com/tools/aapt2
- **Android Developers - apksigner: **https://developer.android.com/tools/apksigner
- **Android Developers - Prepare your app for release: **https://developer.android.com/studio/publish/preparing
- **Android Developers - App Bundle FAQ / app signing background: **https://developer.android.com/guide/app-bundle/faq
# 20. Immediate Next Actions

1. Create the AndroidIntelligentCompiler repository with the proposed top-level layout.
1. Write a short ADR locking Rust + custom IR + direct DEX as the initial architecture.
1. Choose the exact prototype Android API/minSdk profile and physical ARM64 test device(s).
1. Begin M0 only: DEX primitives, layout, integrity and verification.
1. After M0 passes, implement M1 Hello World packaging and installation.
1. Do not begin AI integration until M1/M2 establish a reliable deterministic compiler contract.
## One-sentence handoff

> Build the smallest deterministic Android compiler kernel first; prove AIC IR -> DEX -> signed APK -> real device, then grow the semantic language, optimizer, Android capabilities, on-device host, and finally the AI intent layer.
