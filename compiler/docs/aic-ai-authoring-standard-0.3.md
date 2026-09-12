# AIC AI authoring standard 0.3

This standard governs AI-authored IR 0.3 projects. Compiler schemas, capability catalogs, and diagnostics remain authoritative.

## Modules and names

- Put screens, reusable components, domain types, services, and resources in their matching canonical directories; keep the root manifest declarative.
- Use stable descriptive PascalCase names for exported screens, components, records, and services, and descriptive snake_case names for state, values, events, view IDs, and resource keys.
- Export only declarations required by another module. Use explicit imports and never invent modules, capabilities, permissions, resources, or framework APIs.
- Keep a screen responsible for its lifecycle, navigation decisions, and screen-local state. Move shared behavior into pure functions, domain modules, components, or typed services according to responsibility.

## Components and state

- Prefer composition over inheritance. Reuse a component when repeated UI represents the same semantic role, not merely similar appearance.
- Give components typed parameters and emitted events. Keep internal state private; let the owning screen control state that affects navigation, persistence, or services.
- Prefer the versioned standard component library for common buttons, labeled inputs, toolbars, progress, empty states, rows, and confirmations.
- Preserve behavior and accessibility when proposing extraction. Reuse detection is advisory: the AI proposes semantic consolidation, the user reviews it, and the compiler verifies the result.

## Accessibility and resources

- Use localized resource keys for user-visible text. Keep default strings complete and locale variants structurally consistent.
- Provide labels for inputs, headings where structurally appropriate, meaningful content descriptions for informative images and progress controls, and scale-safe text. Mark purely decorative images explicitly; never infer decorative intent from a missing description. The compiler automatically gives interactive controls a 48dp minimum touch target and rejects explicit fixed interactive dimensions below 48dp.
- Do not duplicate colors, dimensions, text, or images across screens when a shared resource expresses the same meaning. Do not use filenames or content descriptions as substitutes for visible localized labels.

## AI change discipline

- Begin whole-application work with a reviewable module, navigation, data, service, resource, and capability plan.
- In Guided mode, change only the requested page or feature. In all modes, submit bounded file/symbol operations and declare every touched area.
- Request only dependency-relevant context. Do not rewrite unaffected modules, reformat unrelated declarations, change package identity during a patch, or replace source with a summary.
- Treat service contracts, mocks, and implementations distinctly. State unsupported or unavailable behavior explicitly and cite the missing capability; never fabricate successful backend communication.
- Repair from structured compiler diagnostics. After the bounded retry limit, stop and preserve the last valid project rather than weakening requirements or inventing syntax.

## Formatting and reviewability

- Use canonical formatter output and deterministic declaration ordering. Keep one primary exported screen/component/domain/service declaration per module unless tightly coupled private helpers improve clarity.
- Choose names that express product meaning rather than widget type alone. Keep handlers focused and extract shared or complex logic behind typed interfaces.
- Summaries must explain user-visible behavior, affected modules, capability changes, migrations, and known unsupported portions. All changes remain unapplied until validation succeeds and the user accepts the diff.
