# require：ROMClass → ctor materialize + writeback（现行口径）

本文档描述：模块导出中出现 ROMClass 时，`require()` 的 materialize 行为，以及为什么要保留 writeback。

## 1. 目标

- 模块导出的 class 在 JS 侧应表现为可 `new` 的 constructor function。
- 支持懒初始化：仅在 require/访问导出时 materialize。
- materialize 后写回模块对象，确保后续访问是稳定的 ctor。

## 2. 行为定义

- 当模块对象的导出属性值为 ROMClass 时：
  - 触发 ROMClass materialize，得到 ctor function
  - `DefineProperty` 写回到模块对象（own property），用 ctor 替换 ROMClass 值

## 3. 约束

- 该行为属于 require 语义的一部分，与 `ridl_context_init(ctx)` 的 correctness gate 分离。
- module 模式下，ctor 不应被注入 `globalThis`。

## 4. 相关实现

- `deps/mquickjs-rs/require.c`
  - 调用 `JS_MaterializeModuleClassExports(ctx, obj)`
- 引擎 API（mquickjs）：
  - `JS_MaterializeROMClass(ctx, val)`
  - `JS_MaterializeModuleClassExports(ctx, module_obj)`
