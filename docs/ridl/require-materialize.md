# require：ROMClass → ctor materialize（允许 proto exports）

本文档描述：模块导出中出现 ROMClass 时，`require()` 的 materialize 行为，以及 exports 可见性的语义约束。

## 1. 目标

- 模块导出的 class 在 JS 侧应表现为可 `new` 的 constructor function。
- 支持懒初始化：仅在 require/访问导出时 materialize。
- require() 返回的模块对象应能访问到 exports（own 或原型链）。

## 2. 行为定义

- 当模块对象（或其 prototype）上的导出属性值为 ROMClass 时：
  - 触发 ROMClass materialize，得到 ctor function。

- materialize 的结果**允许**写回到 module prototype（proto exports）。
  - 不要求写回为 module instance 的 own property。
  - 因此：`Object.keys(require("...")).length` 允许为 0，但 `require("...").Foo` 必须可访问。

## 3. 约束

- 该行为属于 require 语义的一部分，与 `ridl_context_init(ctx)` 的 correctness gate 分离。
- module 模式下，ctor 不应被注入 `globalThis`。

## 4. 相关实现

- `deps/mquickjs-rs/require.c`
  - 调用 `JS_MaterializeModuleClassExports(ctx, obj)`
- 引擎 API（mquickjs）：
  - `JS_MaterializeROMClass(ctx, val)`
  - `JS_MaterializeModuleClassExports(ctx, module_obj)`
