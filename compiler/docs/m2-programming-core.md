# M2 programming core

M2 keeps AIC IR version `0.1` and adds a source-located lexer/parser, a separate semantic verifier, typed functions, locals, expressions, structured control flow, and basic optimization.

## Language

- Types are non-null `i32`, `bool`, and `string`. Floating point and decimal are deferred.
- `let` is immutable; `var` is mutable. Parameters and loop variables are immutable.
- Functions use `fn name(parameter: type) -> type`. Every path must return. Recursion and cyclic calls are rejected.
- Integer operators are `+ - * / %`; arithmetic wraps to signed 32 bits. Statically known division or remainder by zero is rejected.
- Comparisons are `== != < <= > >=`; boolean operators are short-circuiting `! && ||`.
- String `+` concatenates. `string(i32)` and `string(bool)` are explicit conversions. String equality compares content.
- `if`/`else` is structured. `for i in start..end` evaluates bounds once and uses an ascending, unit-step, end-exclusive range. `while`, `break`, and `continue` are unsupported.

Diagnostics use stable `AIC` codes and token-level line/column spans. Parsing returns `SyntaxProgram`; `verify` is the only supported conversion to the typed `Program` consumed by later compiler stages.

## Optimization and verification

`aic-cli compile` accepts `--opt-level 0|1` and defaults to level 1. Level 1 folds constants, simplifies short-circuit expressions, and removes constant branches. Both levels preserve the same wrapping integer semantics.

Run desktop checks from `compiler/`:

```powershell
$cargo = "$env:USERPROFILE/.cargo/bin/cargo.exe"
& $cargo fmt --all --check
& $cargo test --workspace
& $cargo clippy --workspace --all-targets -- -D warnings
& $cargo run -p aic-cli -- compile --input testdata/compute.aic --output-dir testdata/generated/m2/o1 --profile android-35 --opt-level 1
```

With Android SDK 35, build-tools 35.0.0, and one authorized ARM64 device:

```powershell
./scripts/build-m2.ps1
./scripts/device-smoke-m2.ps1
./scripts/device-smoke-m2-acceptance.ps1
```

The basic smoke test installs both optimization levels and requires the visible text `Result: 36` with no crash-buffer entry. The acceptance script builds and tests the numeric, dynamic-string, multi-function, string-equality, and short-circuit fixtures at both optimization levels. It retries UI hierarchy capture, requires exact visible output, checks the crash buffer, and writes evidence beside each generated artifact.

## Runtime lowering

The M2 DEX backend emits every verified user function as a private static method using its actual name and prototype. User-function and `onCreate` expressions execute as runtime DEX instructions. The evaluator remains an independent semantic oracle for differential tests; production DEX generation does not call it. Structural tests require runtime methods and relevant opcodes and reject complete precomputed display strings.

Runtime instructions are represented by a typed low-level IR with typed registers and symbolic labels. Its two-pass assembler validates register kinds and ranges, detects duplicate or missing labels, resolves forward and backward branches, and rejects branch offsets outside the supported DEX range. The reference loop is emitted through this assembler rather than handwritten offsets.

User-function bodies are lowered from verified statements and expressions rather than a function-specific instruction template. The class builder emits every verified function with its generated prototype, code item, sorted class-data entry, and cross-function call resolution. String parameters and returns use reference registers and object result/return instructions. Runtime conversion uses the correct `String.valueOf(I)` or `String.valueOf(Z)` overload; concatenation uses `StringBuilder`; equality uses `String.equals(Object)` with its `Z` result.

`onCreate` is lowered from verified statements in source order. Programming locals and Android view references are tracked separately, and actual expression results are passed to `TextView.setText`. There is no function-name, literal-argument, prefix, or expected-result extraction in production lowering.

Boolean `&&` and `||` are lowered with a branch after the left operand, so the right operand executes only when required. Structural regression coverage places the conditional branch before a potentially failing division in the right operand.

## M2 closure evidence

The complete closure record is in [m2-acceptance-evidence.md](m2-acceptance-evidence.md). M2 passed formatting, workspace tests, Clippy with warnings denied, deterministic DEX comparison for the complete valid corpus, stable-diagnostic invalid fixtures, and ten physical-device executions covering both optimization levels.
