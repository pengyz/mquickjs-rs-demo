<!-- planning-meta
status: 未复核
tags: build, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
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

- RIDL user classes receive **final numeric** class ids at build-time.
- Rust/C glue needs a stable, compilable mapping from symbolic names to numeric
  ids.

Behavior:

- `ridl-tool` generates `mquickjs_ridl_api.h` containing:
  - `#define JS_CLASS_<...> (JS_CLASS_USER + <int>)`
- `mquickjs_ridl_register.h` includes `mquickjs_ridl_api.h`, and mquickjs ROM
  generation consumes `mquickjs_ridl_register.h`.

Consumers:

- `deps/mquickjs-build` ensures `mquickjs_ridl_api.h` is available in the
  `${include_dir}` it produces.
- Rust code should consume/parse `mquickjs_ridl_api.h` (or its derived Rust
  constants module) and must not rely on `mqjs_ridl_class_id.h` / `RIDL_CLASS_*`.

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
