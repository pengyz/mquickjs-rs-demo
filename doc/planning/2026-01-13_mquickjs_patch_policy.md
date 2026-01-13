# 2026-01-13 mquickjs submodule patch policy

## Goal

We accept small, auditable patches in the `deps/mquickjs` submodule when required
by RIDL / Rust glue integration, especially when the required data is only known
at **build time** inside the mquickjs ROM generation tool.

This document describes:

- what is allowed to be patched
- how to keep the patch small and maintainable
- how to upgrade / rebase to upstream

## Scope (allowed patches)

Allowed patches must satisfy all of the following:

1. **Build-time only**: changes should affect host tools / generated headers,
   not runtime semantics of the embedded engine.
2. **Minimal surface**: keep the diff small and localized.
3. **Stable outputs**: generated outputs must be deterministic.
4. **No RIDL semantics** in mquickjs: mquickjs may expose build-time facts
   (e.g. assigned class ids), but parsing RIDL or generating glue stays in our
   `ridl-tool`.

## Current patch(es)

### Patch: export RIDL class id mapping

Files:

- `deps/mquickjs/mquickjs_build.c`

Rationale:

- RIDL user classes (`RIDL_CLASS_*`) receive **final numeric** class ids at
  build-time.
- Rust glue (bindgen consumers) needs a stable, compilable header that maps
  `RIDL_CLASS_* -> <int>`.
- This mapping is authoritative inside the ROM generation tool.

Behavior:

- Add `-c` option to the `build_atoms()` host tool to print `mqjs_ridl_class_id.h`
  to stdout.
- The header contains an `enum { RIDL_CLASS_xxx = <int>, ... }`.
- When no RIDL classes are present, emit `RIDL_CLASS__DUMMY = 0` to keep the
  header valid.

Consumers:

- `deps/mquickjs-build` writes the tool output to
  `${include_dir}/mqjs_ridl_class_id.h`.
- `deps/mquickjs-rs` bindgen includes it via `-include mqjs_ridl_class_id.h`.

## Upgrade / rebase procedure

1. Update the `deps/mquickjs` submodule commit to the desired upstream.
2. Re-apply the patch by rebasing (preferred):

   - `git -C deps/mquickjs fetch`
   - `git -C deps/mquickjs rebase <new_upstream_commit>`

3. If conflicts happen, keep the behavior identical and re-run verification.
4. Verify outputs and build:

   - `cargo run -p xtask -- build-tools`
   - `cargo clean && cargo test`
   - `cargo run -q -- tests`

## Notes

- If future upstream provides a native header for class id mapping, remove this
  patch and switch consumers to the upstream facility.
