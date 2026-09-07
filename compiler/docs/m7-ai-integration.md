# M7 AI integration

The Android host now supports AI Create and AI Edit. Under **AI settings**, select OpenAI, Claude, OpenRouter, Hugging Face, Ollama, or llama.cpp. The form shows only the credential and server URL fields used by that provider. OpenAI remains the default and uses the pinned `gpt-5.4-mini-2026-03-17` snapshot with provider-side response storage disabled.

Cloud adapters follow their official interfaces: OpenAI Responses, Anthropic Messages, OpenRouter chat completions, and Hugging Face Inference Providers chat completions. All request strict `aic.model-proposal/1` structured output. Model support still depends on the chosen cloud model; a provider error becomes bounded repair-loop diagnostics.

Ollama uses `POST /api/chat` with its JSON-schema `format`; llama.cpp uses its OpenAI-compatible `POST /v1/chat/completions` and schema-constrained `response_format`. Enter a URL reachable from the phone. This workstation currently exposes Ollama 0.33.3 with `llama3.2:latest`, which is the host's initial local default. With USB debugging, map PC services to the phone before using the loopback defaults:

```powershell
adb reverse tcp:11434 tcp:11434 # Ollama
adb reverse tcp:8080 tcp:8080   # llama.cpp
```

Alternatively, bind the server to the PC's private LAN interface and enter a URL such as `http://192.168.1.9:11434`. The host rejects cleartext URLs to public addresses. Ollama requires a locally installed model name. llama.cpp requires `llama-server` to be running with a chat-capable GGUF model and an alias matching the configured model when applicable.

This repository includes a helper configured for the detected `F:\AI` llama.cpp installation and Qwen3 model:

```powershell
./compiler/scripts/start-local-ai.ps1 -Provider llamacpp -AdbReverse
./compiler/scripts/start-local-ai.ps1 -Provider ollama -AdbReverse
```

The host makes at most three provider calls. Every response must match `aic.model-proposal/1`; patch responses must preserve the package and must change the source. A model response that claims an action while returning the original program is rejected as `AIC7006` and repaired. Each candidate runs through native parsing, schema/semantic validation, capability enforcement, optimization, and DEX lowering without writing build artifacts. Compiler diagnostics, including stage, code, message, source location, and rejected output, become repair context for the next attempt. Exhaustion leaves the current project untouched.

After a request starts, the prompt editor is cleared. A dedicated **Clear** button sits at its right edge. **AI history** displays the 25 most recent private provenance entries with prompt, create/edit action, provider/model, attempt, proposal summary, and validator result. It deliberately omits credentials and full generated source; history can be cleared after a confirmation prompt.

A successful candidate opens a bounded textual diff with its summary and declared semantic areas. Only **Apply** writes it to private project storage. The ordinary Build, Install, and Launch controls then run the M6 pipeline. The model has no interface to package assembly, signing, installation, or launch.

Contracts:

- `compiler/schema/aic-ir-0.1.ebnf`
- `compiler/schema/model-proposal-1.json`
- `host/app/src/main/assets/ai/aic-ir-0.1.md`

Build from the repository root:

```powershell
./compiler/scripts/build-m7.ps1
```

For device acceptance, install the host, configure an API key in the host UI, and evaluate one create prompt plus the patch prompts `add a reset button`, `store history`, and `rename the title`. Review each diff before applying, then build, approve installation, launch, and exercise the changed behavior. API use requires network access; compilation and packaging after a proposal is applied remain offline.

Each provider's credential is stored separately under one non-exportable Android Keystore key. Private provenance is stored under `files/ai/provenance.jsonl`. It is size-bounded, is excluded from archives, and never contains credentials, signing material, or APK bytes.
