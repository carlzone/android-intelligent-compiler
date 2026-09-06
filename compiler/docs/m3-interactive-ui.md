# M3 interactive UI

M3 extends AIC IR `0.1` with activity-owned typed `state`, `on_click(button)` handlers, horizontal/vertical `LinearLayout`, `TextView`, `Button`, integer-oriented `EditText`, `ScrollView`, `android.get_text`, `android.set_text`, `valid_i32`, and validated `i32` conversion. `android.set_layout` supports `match_parent`/`wrap_content` width and height plus integer weight `0` or `1`; `android.set_text_color` and `android.set_background_color` accept `#RRGGBB` or `#AARRGGBB` colors. Layout and colors are lowered to framework calls without custom resources.

Generated activities implement `View.OnClickListener`. State and view references are private instance fields, initialized by `onCreate`; one generated `onClick(View)` method dispatches by view identity. Handlers explicitly update visible properties. There are no closures or implicit data binding.

`valid_i32` accepts the complete signed 32-bit decimal range, including `-2147483648`, before `i32` conversion. Out-of-range and malformed values produce `Invalid input` without a parsing exception. Arithmetic retains M2 wrapping semantics.

Activity recreation rebuilds the view tree and resets all state declarations. State persistence is deferred to M4.

Build and desktop validation from `compiler/`:

```powershell
./scripts/build-m3.ps1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run `./scripts/device-smoke-m3.ps1` with one authorized ARM64 device. The counter path is automated. Calculator acceptance enters two signed integers and exercises Add, Subtract, Multiply, Divide, invalid input, and zero divisor at optimization levels 0 and 1, then checks the crash buffer.
