# mquickjs-demo

QuickJS + Rust + RIDL demo

## Build

```bash
# 1) prepare: build tools + generate RIDL aggregates + build QuickJS base/ridl outputs
# Default behavior:
# - auto-detect Cargo.toml from nearest mquickjs.ridl.toml (otherwise error)
# - try cargo unit-graph if available (nightly), otherwise fallback
cargo run -p ridl-builder -- prepare

# 2) build app
cargo build
```

## Test

```bash
cargo test
cargo run -p ridl-builder -- selftest-gc-mark
cargo run -- tests
```

## Notes

### base vs ridl

`ridl-builder prepare` builds two QuickJS output variants:

- **base**: without RIDL extensions (used for core crates / tests that must not depend on js_* symbols)
- **ridl**: with RIDL extensions (used for the app binary and JS integration tests)

`mquickjs-sys` selects which one to link via feature `ridl-extensions`.

### Troubleshooting

- **Missing mquickjs build outputs** (panic from `deps/mquickjs-sys/build.rs`)

  Run prepare first:

  ```bash
  cargo run -p ridl-builder -- prepare
  ```
