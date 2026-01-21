# ridl_context_init(ctx)：RIDL 扩展初始化（现行口径）

本文档定义：为什么 `ridl_context_init(ctx)` 是 RIDL 扩展 correctness 的唯一入口，以及它需要完成哪些工作。

## 1. 目标

- 将 RIDL 扩展的正确性收敛到**单一入口**：`ridl_context_init(ctx)`。
- 使 proto var 安装不依赖 global ctor 查找，也不依赖 ROMClass materialization/hook。

## 2. `ridl_context_init(ctx)` 的职责闭包

在每个 `JSContext` 创建后（且引擎具备 ridl-extensions 能力时）调用：

1) 安装/初始化 per-context 扩展（ctx-ext）：
- 设置 `RidlCtxExtVTable`
- 分配 `CtxExt`（包含所有 singleton slots）
- 将其指针写入 `JSContext` 的 user_data/ext 存储

2) 初始化 singleton slots：
- 通过聚合层生成的 slot 初始化顺序调用各 singleton constructor

3) 安装 proto vars（JS-only prototype fields）：
- 对每个 class_id：确保 prototype 对象存在
- 将字段以 data property 的形式写入 prototype

## 3. 为什么不能依赖 global ctor 查找

- module 模式要求：不污染 `globalThis`。
- 因此 proto var 安装不能通过 `globalThis["Ctor"].prototype` 获取 prototype。

## 4. 为什么不能依赖 ROMClass materialization 路径

- ROMClass materialize 发生在 `require()` 访问导出时，是“按需”的。
- correctness gate 需要在 ctx 初始化时**确定性完成**，不应依赖用户是否触发 require/materialize。

## 5. 引擎侧最小接口需求

为了在不触发 ctor/materialize 的前提下写入 prototype，需要一个最小 escape hatch：

- `JS_EnsureClassProto(ctx, class_id) -> JSValue`
  - 若 `ctx->class_proto[class_id]` 为空，则创建默认继承自 `Object.prototype` 的 prototype 并写回
  - 返回 prototype 或 `JS_EXCEPTION`

## 6. 相关文件

- 生成模板：`deps/ridl-tool/templates/ridl_context_ext.rs.j2`
- 生成输出：`$OUT_DIR/ridl_context_ext.rs`
- App include：`src/ridl_context_init.rs`
- 引擎 API：`deps/mquickjs/mquickjs.h` / `deps/mquickjs/mquickjs.c`
