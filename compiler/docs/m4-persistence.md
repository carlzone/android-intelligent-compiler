# M4 persistence and platform capabilities

M4 adds explicit `persistence.key_value` and `persistence.sqlite` capability declarations, typed `preference` keys, version-1 `database`/`table`/column declarations, text-oriented `android.text_input`, and verified persistence operations.

The expression surface is `preference.get(key)`, `database.insert(table, column: value, ...)`, `database.exists(table, id)`, and `database.get(table, id, column, default)`. Statements are `preference.set(key, value)`, `database.update(table, id, column: value, ...)`, and `database.delete(table, id)`. Inserts return the generated `i32` row ID. Raw SQL and nullable values are unsupported.

Generated activities initialize app-private SharedPreferences and SQLite fields in `onCreate`, create declared tables idempotently, and use compiler-generated parameterized SQLite statements for CRUD. Neither persistence capability emits a manifest permission. `build-profile.txt` records the resolved capability and permission sets.

Build and validate from `compiler/`:

```powershell
./scripts/build-m4.ps1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/device-smoke-m4.ps1 -SkipBuild
```

Schema version 1 has no migration mechanism. Clear application data after changing a schema.
