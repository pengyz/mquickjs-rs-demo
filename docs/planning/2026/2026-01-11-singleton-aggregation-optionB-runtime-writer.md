<!-- planning-meta
status: 未复核
tags: context-init, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：A2 singleton 聚合初始化 — 方案 B（在 mquickjs-rs 引入运行时 SlotWriter）

> 日期：2026-01-11
> 状态：已确认（按用户决策）

## 目标
解决当前编译失败（模块 crate 引用了应用侧 `CtxExt` 类型）并完成 A2 singleton 聚合初始化：
- 在 `deps/mquickjs-rs` 增加一个稳定的运行时抽象，让 **模块生成代码** 可以“按 slot_index 写入单例槽位”，而不需要知道应用 crate 的 `CtxExt` 具体类型。
- ridl-tool 生成的 per-module 初始化 API 改为依赖该运行时抽象。

## 背景/问题回顾
当前失败原因：
- per-module 生成的 `ridl_module_api.rs` 里出现了：
  `pub fn ridl_module_context_init(ext: &mut crate::generated::...::CtxExt)`
- 但模块 crate 内并不存在 `CtxExt`（它仅存在于应用侧聚合输出）。

结论：模块生成物不能引用应用侧类型。

## 方案 B：在 mquickjs-rs 引入稳定运行时写入接口

### 运行时 API（新增）
在 `mquickjs_rs::ridl_runtime` 下新增：

- `pub trait RidlSlotWriter`
  - `unsafe fn set_slot(&mut self, slot_index: u32, ptr: *mut c_void, drop_fn: unsafe fn(*mut c_void)) -> Result<(), RidlSlotSetError>`
- `pub struct RidlCtxExtWriter`
  - 持有 `ext_ptr: *mut c_void`
  - 通过现有 vtable + `ridl_get_erased_singleton_slot(ext_ptr, slot_index)` 找到 `ErasedSingletonSlot`
  - 再尝试写入 slot：
    - **如果 slot 已经 set**：返回 `Err(RidlSlotSetError::AlreadySet { slot_index })`（明确报错）
    - 如果 vtable 未安装/slot 不存在：返回对应错误

复用现有基础设施：
- `mquickjs_rs::ridl_ext_access::{ridl_get_erased_singleton_slot, RidlCtxExtVTable, ridl_set_ctx_ext_vtable}`
- `mquickjs_rs::ridl_runtime::ErasedSingletonSlot`

### 生成代码（变更点）

#### 1) 模块侧 API（per-module ridl_module_api.rs）
模块 crate 内生成：

```rust
pub fn initialize_module() {
    crate::generated::symbols::ensure_symbols();
}

pub fn ridl_module_context_init(w: &mut dyn mquickjs_rs::ridl_runtime::RidlSlotWriter) {
    let _ = w;
}
```

这样模块只依赖 `mquickjs-rs` 的运行时类型，不依赖应用 crate。

#### 2) 应用侧聚合初始化（aggregated ridl_context_init.rs）
应用侧仍然拥有自己的 `CtxExt`（聚合生成），并安装 vtable。
slot 填充流程改为：

- **顺序已确认**：先分配 `CtxExt` 并存入 `ContextInner`（得到稳定 `ext_ptr`），再通过 `RidlCtxExtWriter(ext_ptr)` 调用各模块填充。

伪码：

```rust
// 1) 安装 vtable（once per process）
unsafe { mquickjs_rs::ridl_ext_access::ridl_set_ctx_ext_vtable(&RIDL_CTX_EXT_VT); }

// 2) 分配 ext 并存入 ContextInner（得到 ext_ptr）
let ext = CtxExt::new();
let ext_ptr = Box::into_raw(Box::new(ext)) as *mut c_void;
unsafe { h.inner.set_ridl_ext(ext_ptr, drop_ctx_ext); }

// 3) 用 writer 调模块 init 填槽
let mut w = unsafe { mquickjs_rs::ridl_runtime::RidlCtxExtWriter::new(ext_ptr) };
{%- for m in module_inits %}
    {{ m.crate_name }}::ridl_module_context_init(&mut w);
{%- endfor %}
```

> 说明：`initialize_module()` 仍用于“确保 C 侧 symbols 被强引用/注册”。slot 填充走 `ridl_module_context_init`。

### 覆盖/冲突策略（已确认）
- 若某个 slot 已被 set，再次 set：**必须失败并明确报错**（返回 `Err(AlreadySet { slot_index })`）。
- 不采用静默跳过，也不只 debug_assert。

## 安全性约束
- 模块 init 只负责填槽，不调用 JS API。
- `drop_fn` 必须匹配分配方式。
- 写入必须在 `ridl_set_ctx_ext_vtable` 完成后进行。

## 具体工作项（接下来实现）
1. 在 `deps/mquickjs-rs` 实现 `RidlSlotWriter` / `RidlCtxExtWriter` / `RidlSlotSetError`。
2. 修改 ridl-tool：
   - `generate_module_api_file()`：生成 writer 签名，移除 `CtxExt`。
   - 聚合模板 `rust_context_init_aggregated.rs.j2`：调用 `{{crate}}::ridl_module_context_init(&mut w)`，不再硬编码 `initialize_module(&mut ext)`。
3. 重新生成输出（沿用现有 xtask/build.rs 流程）并修复编译问题。
4. 补充/调整测试覆盖：
   - 编译层面：模块 API 签名符合预期
   - 运行层面：创建 Context 触发聚合 init，不 panic；重复初始化路径可覆盖“AlreadySet”错误
5. 跑完整 build/test，确保全绿。
