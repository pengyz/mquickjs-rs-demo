# Plan: migrate to thin vtable + strict glue/impl split

Date: 2026-01-12

Related spec:
- `docs/ridl/thin-vtable-glue-impl-split.md`

## Goal

Implement the spec across:

- `ridl-tool`: generation changes to make ABI surface thin and enforce glue/impl responsibilities.
- `app` aggregation: ctx-ext + slot indices + per-context init using `RidlErasedSingletonVTable` only.
- `stdlib` module: refactor to match the glue/impl split and remove legacy fat vtable usage.

Constraints:

- C API registration cannot happen at runtime.
- A crate is treated as a RIDL module only if its `src/` contains at least one `*.ridl`.

## Current baseline (observed)

- `mquickjs-rs` already provides:
  - `RidlErasedSingletonVTable` (create/drop)
  - ctx-ext vtable (`RidlCtxExtVTable`) and slot accessor
  - `ErasedSingletonSlot`
- `ridl-tool` currently has two competing per-context init approaches:
  - singleton aggregate (Option A): `ridl_ctx_ext.rs` (good direction)
  - legacy template: `templates/ridl_context_init.rs.j2` generating a fat C ABI vtable header (to be removed from the active path)
- `stdlib` currently mixes glue concerns into impl-side code.

## Approach (high level)

1. Make the aggregated per-context init exclusively slot-based:
   - install ctx-ext vtable
   - allocate `CtxExt`
   - fill slots by calling each module’s `RIDL_*_SINGLETON_VT.create()` and registering `drop`
   - drop all in ctx finalizer

2. Generate slot index constants (`ridl_slot_indices.rs`) and require module glue to reference them.

3. Refactor module generation for singletons so:
   - `*_glue.rs` does all JS conversions, ctx-ext slot lookup, and exception creation.
   - `*_impl.rs` contains only pure Rust traits/structs/functions.

4. Remove/retire the fat per-singleton vtable header path.

## Work items

### 0) Documentation alignment

- Ensure `docs/ridl/thin-vtable-glue-impl-split.md` stays authoritative during changes.

### 1) ridl-tool: aggregated ctx-ext + slot indices

**Deliverables**

- Generate `ridl_slot_indices.rs` alongside `ridl_ctx_ext.rs`.
- Update `templates/rust_ctx_ext.rs.j2` to include/re-export the slot indices.

**Notes**

- Slot ordering must be derived from the same sorted singleton list used to generate `CtxExt`.

### 2) ridl-tool: aggregated per-context init (thin)

**Deliverables**

- Implement/extend `rust_context_init_aggregated.rs.j2` (or equivalent) to:
  - install `RidlCtxExtVTable` once per process (idempotent)
  - allocate `CtxExt` and store it in `ContextInner` (use existing `ridl_ext_ptr` plumbing)
  - create a `RidlCtxExtWriter` and set each singleton slot with:
    - `RIDL_*_SINGLETON_VT.create()` pointer
    - `RIDL_*_SINGLETON_VT.drop` drop fn
  - register a JSContext finalizer that calls `CtxExt::drop_all()`

**Changes**

- Make `deps/ridl-tool/src/generator/context_init.rs` unused/obsolete for singleton lifecycle.
- Keep/retain `ridl_module_api.rs` if needed for symbol keep-alive, but ensure singleton init is slot-based.

### 3) ridl-tool: module singleton glue fixes (slot constants)

**Deliverables**

- Update `templates/rust_glue.rs.j2` for singleton methods/properties to use `RIDL_SLOT_<NAME>` constants instead of embedding numeric `slot_index`.
- Decide how module crates access slot constants:
  - Preferred: module build.rs includes generated `ridl_slot_indices.rs` from the same plan output (shared under `out/ridl/`), mirroring current temporary convention for `ridl_ctx_ext.rs`.

### 4) stdlib module refactor to spec

**Deliverables**

- `stdlib_glue.rs`:
  - contains all ctx-ext lookup, JS conversions, and JS exception creation.
  - calls impl trait/struct methods only.
- `stdlib_impl.rs`:
  - contains only pure Rust implementation (no QuickJS C APIs, no ctx-ext slot access).
- Export only thin singleton vtable:
  - `RIDL_CONSOLE_SINGLETON_VT: RidlErasedSingletonVTable`.
- Remove legacy fat `RidlConsoleVTable` usage from active path.

### 5) Tests

Add smoke tests that cover:

- calling `ridl_context_init` + invoking `console.log/error` does not crash and returns `undefined`.
- reading `console.enabled` returns a boolean.
- dropping the context triggers singleton drop exactly once.

Test location/framework must follow existing repo conventions.

### 6) Verification

- Run the project’s established build/test commands (to be confirmed from repo docs/scripts) and fix failures.

## Risks / open questions

1) Where is ctx-ext pointer stored today?
- Must confirm `ContextInner.ridl_ext_ptr` storage and how finalizer is wired.

2) Slot constants visibility to module crates
- We need a stable convention for sharing generated `ridl_slot_indices.rs` with module crates without depending on the app crate.

3) String conversion memory management
- Current glue uses `JS_ToCString` without a corresponding free helper. Track as known limitation or extend bindings.

## Exit criteria

- No active code path uses the fat per-singleton vtable header.
- `stdlib` follows glue/impl split strictly.
- Singleton lifecycle is fully slot-based via `RidlErasedSingletonVTable`.
- Added smoke tests pass.
