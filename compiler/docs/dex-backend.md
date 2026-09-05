# M0 DEX backend

The M0 backend emits deterministic DEX version 035 files. Its only supported input is one validated class descriptor. The resulting class is public, extends `java.lang.Object`, and contains one public no-argument constructor whose code invokes `Object.<init>` and returns.

Supported sections are the header, string IDs/data, type IDs, one no-parameter prototype, method IDs, one class definition, one class-data item, one code item, and the map list. Index construction, layout, offsets, and integrity fixups are centralized in the writer. Strings and identifier tables are sorted and deduplicated.

Unsupported in M0: fields, interfaces, annotations, debug info, exceptions, static values, parameter lists, arbitrary instructions/methods, APK packaging, resources, signing, and Android framework lowering. Invalid class descriptors and encoding size failures return typed errors.

Generate the fixture from `compiler/` with:

```text
cargo run -p aic-cli -- emit-minimal
```

Verification uses the independent structural integration test in `aic-dex/tests/verify_minimal.rs`, which parses the header, identifier tables, map, class data, and code item without using the encoder's layout implementation. If Android build tools are installed, `dexdump testdata/generated/minimal.dex` is an additional external oracle.

Run the complete M0 gate on Windows with `./scripts/verify-m0.ps1`. It checks formatting, tests, linting with warnings denied, two-build byte equality, and invokes `dexdump` when that external tool is discoverable. The repository CI runs the clean Rust and reproducibility portions on every push and pull request.
