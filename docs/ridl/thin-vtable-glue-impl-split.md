# Thin vtable + glue/impl split（RIDL）

本文档是本仓库 RIDL 生成代码的长期设计规范。

## Goals

- Keep cross-crate / cross-language ABI surface **minimal and explicit**.
- Make `ridl-tool` code generation **mechanical and predictable**.
- Enforce a strict separation:
  - `*_glue.rs`: QuickJS-facing **C ABI** entrypoints + JS↔Rust conversions + dispatch
  - `*_impl.rs`: **pure Rust** business logic surface (traits/structs/functions)
- Avoid relying on Rust `dyn Trait` ABI across FFI boundaries.
- Preserve the core repo constraint: **C-side registration cannot happen at runtime**.

Non-goals (v1):

- General callback/closure bridging.
- Fully featured conversions for complex RIDL types (`map`, `union`, `optional`, user-defined structs) unless explicitly implemented.

## Terminology

- **glue**: Generated code that exposes QuickJS-callable C ABI functions and performs argument/return conversions.
- **impl**: Pure Rust implementation API that glue calls.
- **singleton**: RIDL item representing a per-`JSContext` (per-context) instance stored in RIDL ctx-ext slots.
- **ctx-ext**: Per-context extension state allocated/owned by the app (aggregated), containing singleton slots.
- **thin vtable**: Minimal per-singleton vtable used only for creating and dropping singleton instances.

## Supported RIDL items (shape → rules)

RIDL AST types are defined in `deps/ridl-tool/src/parser/ast.rs`.

### 1) Free functions (`IDLItem::Function`)

JS shape: global function `foo(...)`.

Rules:

- glue generates one C ABI entry per function:
  - `js_<fn>(ctx, this_val, argc, argv) -> JSValue`
- glue is responsible for:
  - arity/type checks
  - `argv` extraction and conversion to Rust types
  - calling `crate::impls::<fn>(...)`
  - converting return value to `JSValue`
  - throwing JS exceptions on errors
- impl exposes a pure Rust function:
  - `pub fn <fn>(<rust params>) -> <rust ret>`
- impl MUST NOT:
  - touch QuickJS C APIs (`JS_*`), `argc/argv`, `ContextHandle`, ctx-ext slots

### 2) Singletons (`IDLItem::Singleton`)

JS shape: global object `console.log(...)`, `console.enabled`.

Rules:

- glue generates C ABI entrypoints for:
  - methods: `js_<singleton>_<method>(ctx, this_val, argc, argv) -> JSValue`
  - readonly property getters: `js_<singleton>_get_<prop>(...) -> JSValue`
  - (future) setters for readwrite properties
- glue responsibilities:
  1. convert JS arguments
  2. locate singleton instance via ctx-ext slot
  3. call impl-side API
  4. convert return value and/or throw JS exceptions

- impl responsibilities:
  - define the pure Rust surface (trait or struct API)
  - provide a constructor used by the singleton vtable `create`
  - MUST NOT do JS conversions, slot access, or JS exception creation

#### Singleton lifetime model

- Each singleton is **per `JSContext`**.
- The app-level aggregated context initializer allocates per-context ctx-ext storage.
- Singleton allocations are stored in ctx-ext slots and dropped from ctx finalizer.

### 3) Interfaces (`IDLItem::Interface`)

Interfaces can be modelled as either:

- Namespace-like grouping (no instance): treat as free functions
- Instance-like object methods: treat as class instance methods

Rule:

- The impl layer MUST NOT receive raw `JSValue this_val`.
- glue must extract receiver/instance and pass a Rust reference/pointer.

### 4) Classes (`IDLItem::Class`)

JS shape: `new Foo(...)`, `foo.method(...)`, property access.

Rules:

- glue generates:
  - constructor entrypoint
  - method entrypoints
  - property getter/setter entrypoints
- glue is responsible for mapping `this_val` to a Rust instance:
  - store `*mut Foo` (or another thin pointer) in JS object opaque
  - ensure finalizer drops the Rust allocation exactly once
- class instances do **not** use ctx-ext singleton slots.
- impl defines pure Rust `struct Foo` and methods.

### 5) Types (`Enum`, `StructDef`, `Using`, `Import`)

- These are type-level items.
- Conversions are performed in glue when referenced by callable APIs.
- If a type conversion is not supported, the generator should fail fast (compile-time error) rather than generating partial/unsafe behavior.

## Code generation outputs and responsibilities

### Module crate (per RIDL module; generated into module `OUT_DIR`)

- `<module>_glue.rs`
  - only glue (C ABI entrypoints, conversions, receiver/singleton resolution, dispatch to impl)
  - contains `#[no_mangle] pub unsafe extern "C" fn js_*` exports

- `<module>_impl.rs`
  - only impl surface (traits/structs/functions)
  - may contain `todo!()` placeholders in generated skeleton
  - MUST NOT reference QuickJS C APIs or ctx-ext

- `<module>_symbols.rs`
  - `ensure_symbols()` strongly references `js_*` exports to keep them linked

- `ridl_module_api.rs`
  - `initialize_module()` calls `symbols::ensure_symbols()`
  - `ridl_module_context_init(w: &mut dyn RidlSlotWriter)` optional hook for module-provided slot filling

### App crate (aggregated; generated into app `OUT_DIR`)

- `ridl_context_ext.rs`
  - defines `CtxExt` with `ErasedSingletonSlot` fields
  - defines `ridl_ctx_ext_get_slot_by_name(ext_ptr, name_ptr, name_len)` used via `RidlCtxExtVTable`
  - defines `CtxExt::drop_all()`
  - provides `ridl_context_init(ctx)` entrypoint
  - installs the ctx-ext vtable once per process
  - allocates ctx-ext and initializes singleton slots
  - ensures ctx finalizer calls `CtxExt::drop_all()`

## ABI rules

### 1) Thin singleton vtable (the only required per-singleton ABI)

Use `mquickjs_rs::ridl_runtime::RidlErasedSingletonVTable`:

```rust
#[repr(C)]
pub struct RidlErasedSingletonVTable {
    pub create: unsafe extern "C" fn() -> *mut core::ffi::c_void,
    pub drop: unsafe extern "C" fn(*mut core::ffi::c_void),
}
```

Each singleton MUST export:

- `pub static RIDL_<NAME>_SINGLETON_VT: RidlErasedSingletonVTable`

The vtable MUST NOT include method function pointers.

### 2) Slot storage pointer MUST be thin

- `ErasedSingletonSlot` stores `*mut c_void`.
- The stored pointer MUST be a thin pointer.
- DO NOT store a Rust fat pointer (e.g. `*mut dyn Trait`) directly.

Recommended pattern:

- store a `*mut Box<dyn Trait>` (i.e. pointer to a box) by allocating `Box<Box<dyn Trait>>`
- glue casts slot ptr back to `*mut Box<dyn Trait>`, then dereferences to `&mut dyn Trait`

### 3) Glue C ABI entrypoints

Each exported JS entrypoint uses QuickJS C function signature:

```rust
#[no_mangle]
pub unsafe extern "C" fn js_<name>(
    ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
    this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    argc: std::os::raw::c_int,
    argv: *mut mquickjs_rs::mquickjs_ffi::JSValue,
) -> mquickjs_rs::mquickjs_ffi::JSValue
```

All argument parsing and conversion happens in glue.

## Singleton name-key（替代 slot index）

- slot index 在多 crate / 各自 build.rs 的模型下很难保证一致（无法可靠共享同一份聚合产物）。
- 因为 singleton 挂到 JS global 上，其名字天然应当全局唯一，因此改为用 name-key 定位。
- `RidlCtxExtVTable` 提供 `get_slot_by_name(ext_ptr, name_ptr, name_len)`。
- module glue 直接使用编译期字面量名（如 `b"console"`），不再引用任何共享 index 常量。

## Error and exception policy

- glue converts parameter/type errors into JS `TypeError` (or a chosen stable QuickJS error class).
- impl does not throw JS exceptions.
- If impl returns `Result<T, E>` in the future, glue will translate `Err` into JS exception.

## Required smoke tests

For each supported item category, maintain a minimal smoke test:

- singleton:
  - after `ridl_context_init(ctx)`, slot is set
  - calling method works (no panic/UB)
  - ctx drop triggers singleton drop exactly once

- free function:
  - arity/type checks throw TypeError
  - return conversion is correct

- class (when implemented):
  - ctor creates instance
  - methods dispatch to impl
  - finalizer drops instance exactly once

## Current deviations (as of 2026-01)

- `ridl-tool/templates/ridl_context_init.rs.j2` generates a per-singleton "fat" C ABI vtable header (method fn pointers in a struct). This conflicts with the thin vtable goal.
- `ridl-modules/stdlib` currently mixes glue responsibilities (ctx-ext slot access, JSValue conversion, JS exception creation) into impl-side code. This violates the glue/impl split.
- Some glue code hardcodes slot indices rather than referencing generated constants.

## Migration plan (documentation only)

1. Stop generating the fat per-singleton vtable header; switch aggregated context init to:
   - install `RidlCtxExtVTable`
   - allocate `CtxExt`
   - fill singleton slots via `RIDL_*_SINGLETON_VT.create/drop`
   - drop via `CtxExt::drop_all()`

2. 移除 slot index 路径；ctx-ext 提供 `get_slot_by_name`，module glue 直接用 singleton 名字定位。

3. Refactor module crates (starting with `stdlib`) to enforce:
   - `*_glue.rs`: all JS<->Rust conversion + slot/receiver resolution + JS exceptions
   - `*_impl.rs`: pure Rust implementation surface

4. Add/extend smoke tests to cover singleton lifecycle and call paths.
