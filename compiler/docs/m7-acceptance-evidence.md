# M7 acceptance evidence

Status: Complete
Date: 2026-09-07
Device: Xiaomi `2312DRA50G`, serial `e56c4a46`, ARM64, Android 16

Automated evidence:

- Rust workspace tests and Clippy pass, including JNI validation parity and validation without artifact writes.
- Android debug host, instrumentation APK, JVM tests, and lint build successfully.
- Physical-device instrumentation passes all M6 regressions at O0/O1.
- Physical-device M7 tests reject malformed structured output and invented syntax, pass structured diagnostics into a second attempt, accept a repaired locally compilable proposal, terminate after three bad proposals, and preserve the last valid source.
- Package preservation, response size/field limits, a bounded review diff, private provenance, and separation from signing/install APIs are enforced in host code.

Live-provider exit evidence:

- Ollama with the local pretrained `llama3.2:latest` model created a counter application from a natural-language prompt. The proposal was reviewed and applied, then compiled, packaged, installed, launched, and exercised successfully on the reference device.
- Iterative edits successfully persisted the counter between launches, vertically centered the existing controls, renamed the Clear control to Reset while preserving behavior, and renamed the application title to My Persistent Counter.
- Invalid intermediate model output was rejected by schema, parser, semantic, or lowering diagnostics. The repair loop remained bounded and preserved the last valid project source.
- Compiler-owned transformations constrain recognized metadata and layout edits to the requested semantic area when a small local model adds unrelated or invalid syntax.

Post-M7 findings:

- A request for a close button in the upper-right corner is outside AIC IR 0.1: the profile has no gravity/absolute-positioning primitive and no activity-finish operation. Ollama invented unsupported layout coordinates and a fake close action, which local validation correctly rejected.
- Follow-on work should distinguish unsupported intent before generation, expand layout and lifecycle primitives deliberately, and evaluate retrieval, constrained decoding, and optional model specialization against a held-out AIC prompt suite.
