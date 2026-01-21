<!-- planning-meta
status: 未复核
tags: build, context-init, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `context-init` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：singleton 以 name 作为 key（替代 slot index）

日期：2026-01-12

背景：
- 目前的 slot index 方案在“多 crate + 各自 build.rs”模型下非常脆弱：每个 RIDL module crate 都在自己的 `OUT_DIR` 下生成代码，但并没有可靠方式获取 app crate 聚合产物（也就无法共享同一份 index 常量）。
- singleton 的语义通常是“挂到 JS global 对象上的全局单例对象（per JSContext）”，其名字天然应当全局唯一（例如 `console`）。

## 目标

用“singleton name-key”替换跨 crate 的“slot index 合约”：

- app 侧仍然负责 per-context 存储（`CtxExt`）与生命周期（create/drop），并在编译期聚合生成。
- module glue 侧通过 name-key 的 ctx-ext vtable 定位 singleton slot，不再依赖任何共享的 `ridl_slot_indices.rs`。

约束：
- C API 注册不能在运行时发生，只能编译期静态注册。
- 一个 crate 只有在其 `src/` 下存在至少一个 `*.ridl` 时，才被视为 RIDL module。
- 目前不引入 singleton 命名空间（不支持 `std.console` 之类），名字直接是全局唯一标识。

## 设计

### ABI 面

保持 thin erased singleton vtable：

- `RidlErasedSingletonVTable { create: fn() -> *mut c_void, drop: fn(*mut c_void) }`

将 ctx-ext vtable 的访问从：

- `get_slot(ext_ptr, slot_index) -> *mut ErasedSingletonSlot`

改为 name-key：

- `get_slot_by_name(ext_ptr, name_ptr, name_len) -> *mut ErasedSingletonSlot`

说明：
- name 为 singleton 的标识符字节序列（UTF-8/ASCII），如 `"console"`。
- 找不到返回 null。

### 存储模型

`CtxExt` 仍然是一个 struct，包含每个 singleton 对应的 `ErasedSingletonSlot` 字段。

聚合生成器额外生成 name->slot 的映射逻辑（建议生成 `match`）：

- `fn match_singleton_slot_mut(ext: &mut CtxExt, name: &str) -> Option<&mut ErasedSingletonSlot>`
- `ridl_ctx_ext_get_slot_by_name(...)` 内部调用该 match。

选择：本计划实现 `match`（简单、零依赖、性能足够）。

### glue / impl 职责

- glue：QuickJS 参数解析、JS↔Rust 转换、异常抛出、ctx-ext lookup、调用 impl。
- impl：纯 Rust 业务逻辑（trait/struct/fn），不做 JS 转换、slot 访问、异常构造。

glue 查找 singleton 示例：

- `ridl_get_erased_singleton_slot_by_name(ext_ptr, b"console".as_ptr(), 7)`

不再需要共享 `ridl_slot_indices.rs`。

## 工作项

### 1) mquickjs-rs：增加 name-key 访问能力

- 在 `mquickjs_rs::ridl_ext_access` 增加 helper：
  - `ridl_get_erased_singleton_slot_by_name(ext_ptr, name_ptr, name_len) -> Option<*mut ErasedSingletonSlot>`
- 更新 `RidlCtxExtVTable`：增加 `get_slot_by_name` 字段。
- 迁移期可暂时保留旧的 `get_slot(slot_index)` 以便平滑过渡，但生成代码不再使用它。

### 2) ridl-tool：聚合 ctx-ext 生成器

- 更新 `rust_ctx_ext.rs.j2`：
  - 生成 `pub unsafe extern "C" fn ridl_ctx_ext_get_slot_by_name(...)`。
  - 生成 `match_singleton_slot_mut`。
- `CtxExt::drop_all()` 保持不变。
- 停用（或仅保留但不走主路径）slot index 相关文件生成/引用。

### 3) app：聚合 ridl_context_init

- 不再依赖 slot index 常量。
- 依然可直接通过字段初始化 slot：`ext.console.set(ptr, drop)`。

### 4) ridl-tool：module glue 模板

- 移除 `include!(.../ridl_slot_indices.rs)`。
- 将 slot lookup 替换为 name-key 版本：
  - `...ridl_get_erased_singleton_slot_by_name(ext_ptr, b"{{ s.name|lower }}".as_ptr(), {{ s.name|length }})`

### 5) stdlib 清理

- 确保 stdlib 严格遵循 glue/impl 分层。
- 导出的 singleton vtable 仍为 thin。
- 不再出现 slot index 相关依赖。

### 6) 测试

- smoke 覆盖：
  - `ridl_context_init` 后 `console.log/error` 可调用（不 crash）。
  - `console.enabled` 返回 bool。
  - ctx drop 时 singleton drop 恰好一次。

## 迁移/兼容策略

- 迁移期允许旧 index-based API 保留，但生成代码完全切到 name-key。
- 待全 workspace build/test 通过后，再删 dead code。

## 验收标准

- 任意 crate 不再依赖生成的 `ridl_slot_indices.rs`。
- 所有 singleton 访问走 name-key。
- workspace build/test 通过。
