# M11 AI-native modular development and AIC Studio

M11 replaces whole-project model rewriting with a modular, transactional authoring system. It introduces AIC IR 0.3 and `.aicproject` format 3 after the M10 service contracts are stable. IR 0.2 and project formats 1-2 remain readable; migration is deterministic, shown for review, and never rewrites stored data without approval.

## Canonical project organization

The archive contains a root `project.aic`, modules under canonical `screens/`, `components/`, `domain/`, `services/`, and `resources/` paths, and validated binary files under `assets/images/`. The root declares the launcher, included modules, resources, and required capabilities. Archive entries, module resolution, resource IDs, and checksums use deterministic ordering.

Modules use explicit imports and exports. Wildcards, ambiguous symbols, path traversal, dependency cycles, unrestricted inheritance, reflection, and arbitrary framework calls are invalid. Screens own lifecycle and screen state. Components encapsulate reusable UI through typed parameters, private state, validated view output, and emitted events. Services expose bounded typed contracts; an interface, mock, or unavailable implementation must never be presented as a working production backend.

## AI context and transactions

The compiler produces a semantic index of exported signatures, dependencies, navigation, resource references, capabilities, and component usages. The prompt builder supplies the project summary, requested module, direct interfaces, applicable capability entries, and validated examples. It excludes unrelated implementations and can request another indexed module when a dependency is discovered.

AI output uses a versioned patch contract with a base-project hash, per-module hashes, declared touched areas, and bounded operations to add, remove, replace, or rename files and symbols. The host applies a proposal to an isolated snapshot, resolves modules, validates the complete application, and presents per-file changes. A stale hash, invalid operation, or compiler failure rejects the entire transaction and preserves the prior project.

Guided mode limits work to one page or feature and is recommended when evaluation shows that a model is unreliable at project scope. Assisted mode works through an approved plan feature by feature. Autonomous mode accepts a whole-app description, presents an editable application plan, and executes bounded validated patches. Model names are informative only; recommendations use measured schema, compilation, repair, scope-control, and unsupported-capability performance.

## AIC Studio core

The M11 host provides AI, blank, template, import, duplicate, and recent-project entry points. Its responsive workspace includes a project tree, per-module canonical source editor, contextual AI panel, plan review, per-file diffs, and consolidated build and diagnostics status. Phone layouts use navigable panels; expanded layouts may show project, editor, and review context together.

Templates are optional starting points rather than the meaning of a new project. Source remains canonical and directly editable. AI cannot apply, sign, install, launch, or publish without the existing user and compiler-controlled gates.

Synchronized visual/source editing, editable navigation graphs, device-configuration previews, and component galleries are explicitly deferred beyond the M11 core workspace.

## Acceptance evidence

The reference corpus includes a sufficiently large multi-screen project with shared components, resources, domain models, and M10 service contracts. It must prove deterministic output under reordered modules, stable file/source diagnostics, transactional rollback, stale-patch rejection, relevant-context isolation, and interruption recovery.

The same application outcome is evaluated through Guided lower-capability and Autonomous higher-capability workflows. Both must pass identical compiler, accessibility, Android compatibility, and device gates, including explicit informative-versus-decorative image semantics and legal visibility/enabled targets inherited from IR 0.2. Evidence records first-pass compile rate, repair convergence, unintended-file changes, unsupported-capability accuracy, input/output tokens, user review steps, and final behavior.
