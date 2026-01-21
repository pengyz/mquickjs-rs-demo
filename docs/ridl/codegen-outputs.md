# RIDL 聚合产物与职责边界（现行口径）

本文档定义：ridl-tool/ridl-builder 在 app 侧生成的“聚合产物”有哪些，它们分别负责什么。

## 1. 聚合产物清单（app OUT_DIR）

- `mquickjs_ridl_register.h`
  - C 侧编译期注册入口（stdlib 注入）
  - 约束：mquickjs 的注册必须编译期完成

- `ridl_symbols.rs`
  - 聚合 symbols（模块集合、导出符号等）

- `ridl_context_ext.rs`
  - 定义 `CtxExt`（包含所有 singleton slots）
  - 定义 slot 访问与 drop
  - 提供 `ridl_context_init(ctx)`：唯一 correctness gate

- `ridl_bootstrap.rs`
  - 模块 keep-alive / process initialize
  - （可包含对 context init 的再导出或组织）

## 2. 一致性要求：class_id 只能有一个来源

- `mquickjs_ridl_register.h` 与 `ridl_context_ext.rs` 必须共享同一份 class_id 分配信息。
- 生成流程应复用聚合 IR，避免两次独立分配导致不一致。

## 3. 相关实现位置

- ridl-tool：`deps/ridl-tool/src/generator/*`
- ridl-builder：`ridl-builder/src/aggregate.rs`
- app include：`src/ridl_context_init.rs`
