# ADR 0005: M7 AI trust boundary

Status: Accepted, 2026-09-07

M7 defines `ModelProvider` as a provider-neutral request/response boundary. Adapters support OpenAI Responses, Anthropic Messages, OpenRouter chat completions, Hugging Face Inference Providers chat completions, Ollama chat, and llama.cpp's OpenAI-compatible chat endpoint. They request strict structured output using each provider's official schema mechanism. OpenAI uses `store: false` and the pinned `gpt-5.4-mini-2026-03-17` snapshot by default.

Model output is limited to a versioned JSON proposal containing complete AIC source and review metadata. The host validates exact fields and limits, preserves the package during patches, then invokes the same Rust parse, semantic, capability, optimization, and DEX-lowering path used by a normal build. Validation writes no artifacts. A proposal remains separate from project storage until the user reviews and applies it.

The model never receives a filesystem path, signing key, Android Keystore handle, unsigned or signed APK bytes, PackageInstaller session, or launcher authority. Packaging, signing, installation, and launch remain downstream deterministic host operations. API credentials are encrypted by a non-exportable Android Keystore AES-GCM key and are omitted from diagnostics and provenance.

Each attempt records private JSONL provenance: provider/model, provider response ID, schema version, operation, prompt, base-source hash, raw structured proposal, validator result, and timestamp. The log is app-private and excluded from project archives because prompts may contain sensitive content.
