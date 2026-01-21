# RIDL 聚合文件收敛方案（ridl-builder 输出）

日期：2026-01-14

状态：草案（待审阅）

> 目标：将 ridl-builder 生成的 Rust 聚合文件从“多个零散文件”收敛为少数几个（工程上更易读、易 include、易 debug），同时保证：
>
> - 语义不变（同样完成编译期注册与 keep-alive）。
> - 无“模块白名单/硬编码”引入；模块选择仍由 root crate direct deps + `src/*.ridl` 决定。
> - 与现有调用点尽可能兼容，支持渐进迁移。

---

## 1. 背景

当前 ridl-builder 在 `<target_dir>/ridl/apps/<app-id>/aggregate/` 下生成多份 Rust 文件（示例）：

- `ridl_symbols.rs`
- `ridl_slot_indices.rs`
- `ridl_ctx_ext.rs`
- `ridl_context_init.rs`
- `ridl_modules_initialize.rs`
- `ridl_bootstrap.rs`

其中：

- `ridl_symbols.rs` 由 ridl-tool 生成（keep-alive：extern + 取地址）。
- `ridl_slot_indices.rs` 由 ridl-tool shared files 生成（slot index 常量）。
- `ridl_ctx_ext.rs`、`ridl_context_init.rs` 由 ridl-tool singleton aggregation 生成。
- `ridl_modules_initialize.rs`、`ridl_bootstrap.rs` 由 ridl-builder 生成。

问题：

- 文件过多、include 分散：上层需要 include 多个文件/模块，定位困难。
- 生成器边界不清晰：部分文件由 ridl-tool、部分由 ridl-builder，组织上较混乱。

---

## 2. 收敛目标（最终产物）

将 Rust 聚合文件收敛为 **3 个**：

1) `ridl_symbols.rs`（保持独立）
2) `ridl_context_ext.rs`（新增，合并 context 扩展结构与 slot 索引，并提供 ridl_context_init 入口）
3) `ridl_bootstrap.rs`（新增，合并初始化入口与上下文初始化）

C 侧保持 1 个：

- `mquickjs_ridl_register.h`

说明：JSON 快照（`ridl-manifest.json` / `ridl-deps.json` / `ridl-unit-graph.json`）不属于“聚合代码文件”，不在本收敛目标内。

---

## 3. 分类依据（按职责拆分）

### 3.1 keep-alive（linker 保活）

- 现状：`ridl_symbols.rs`
- 特点：
  - 仅声明 `extern "C"` glue entrypoints，并取地址以强制链接保留。
  - 强约束：不得 `use crate::generated::glue::*`，避免重复符号/冲突。

结论：**保持独立文件** 最安全。

### 3.2 运行时支持（CtxExt/slot 协议）

- 现状：
  - `ridl_slot_indices.rs`（slot index 常量/映射）
  - `ridl_ctx_ext.rs`（CtxExt 定义与 slot 访问函数）

二者高度耦合：CtxExt 的字段顺序/访问分支与 slot index 一一对应。

结论：合并为 `ridl_context_ext.rs`。

### 3.3 启动/初始化（process 与 context）

- 现状：
  - `ridl_modules_initialize.rs`：强制拉入各 module crate（initialize_module）
  - `ridl_context_init.rs`：对 JSContext 安装 ridl ctx ext vtable，分配并填充 slots
  - `ridl_bootstrap.rs`：提供统一入口 initialize()，调用 modules::initialize_modules 与 symbols::ensure_symbols

这三者都属于“启动/初始化”职责，且上层一般只关心“调用哪个入口”。

结论：合并为 `ridl_bootstrap.rs`。

---

## 4. 新文件的接口设计

### 4.1 `ridl_context_ext.rs`

内容包含：

- slot indices（原 `ridl_slot_indices.rs`）
- CtxExt 结构与 slot getter（原 `ridl_ctx_ext.rs`）

对外导出建议：

- `pub use crate::ridl_context_ext::CtxExt;`（如需要）
- （保持现有 ridl_context_init.rs 的 `mod ridl_ctx_ext { include!(...) }` 结构也可，但更推荐直接在同文件内引用）

### 4.2 `ridl_bootstrap.rs`

内容包含：

- modules initialize（原 `ridl_modules_initialize.rs`）
- context init（原 `ridl_context_init.rs`，内部引用 `ridl_context_ext::CtxExt`）
- process initialize（原 `ridl_bootstrap.rs`）

对外 API：优先保持兼容，提供原调用点：

```rust
pub mod ridl_bootstrap {
    pub fn initialize();
}
```

并在内部实现：

- `initialize()` 仍调用：
  - `modules::initialize_modules()`
  - `symbols::ensure_symbols()`

此外可额外暴露：

- `pub unsafe fn ridl_context_init(ctx: *mut JSContext)`（若上层需要直接调用）

---

## 5. 迁移策略（一次性切换）

本次选择最干净但改动较大的方式：**由 ridl-tool 直接生成最终 3 个 Rust 文件**。

因此迁移策略为：

- 不生成旧文件名（`ridl_slot_indices.rs` / `ridl_ctx_ext.rs` / `ridl_context_init.rs` / `ridl_modules_initialize.rs` / `ridl_bootstrap.rs`）。
- 同步修改所有消费方（root build.rs / mquickjs-ridl-glue / 任何 include 点）只依赖新文件：
  - `ridl_symbols.rs`
  - `ridl_context_ext.rs`
  - `ridl_bootstrap.rs`

验收要求：

- 改动完成后必须跑全量验证：`cargo test` + `cargo run -- tests`。
- 全量通过后直接落地；不做薄壳兼容，避免长期背负历史包袱。

---

## 6. 实现方案（路线 B：ridl-tool 直接生成）

### 6.1 ridl-tool 生成侧（主要改动）

现状 ridl-tool 通过多个 generator/template 生成：
- `ridl_slot_indices.rs`（shared files）
- `ridl_ctx_ext.rs` / `ridl_context_init.rs`（singleton aggregation）
- `ridl_symbols.rs`（shared files / symbols）

目标改为：ridl-tool 直接生成以下最终文件：

- `ridl_symbols.rs`（保留现状语义，可调整内部结构）
- `ridl_context_ext.rs`（合并 slot indices + CtxExt + slot getters，并提供 ridl_context_init）
- `ridl_bootstrap.rs`（合并：modules initialize + process initialize + context init）

实现要点：

1) 新增一个“聚合模板/生成入口”，负责一次性写出上述文件。

2) 停止写出旧文件：
- `ridl_slot_indices.rs`
- `ridl_ctx_ext.rs`
- `ridl_context_init.rs`

3) `ridl_bootstrap.rs` 需要包含：
- 强制拉入模块：调用各 module crate 的 `initialize_module()`（目前由 ridl-builder 生成，需要迁入 ridl-tool 或让 ridl-tool 接收 crate 列表）
- keep-alive：调用 `symbols::ensure_symbols()`（从 ridl_symbols.rs 引入）
- context init：设置 ridl ctx ext vtable、分配 CtxExt、并通过 runtime writer 写入各 singleton slots

4) 文件依赖关系：
- `ridl_bootstrap.rs` 引用 `ridl_context_ext.rs` 的 `CtxExt` 与 slot getter。
- `ridl_symbols.rs` 尽量自包含，避免额外 `use` 导致重复符号风险。

> 关键决策：modules initialize 的生成位置。
>
> - B1（推荐）：由 ridl-tool 接收模块 crate name 列表并生成 `initialize_modules()`，彻底消灭 ridl-builder 的零散文件输出。
> - B2：仍由 ridl-builder 生成 `initialize_modules()`，但写入到 `ridl_bootstrap.rs`（不再产生独立小文件）。
>
> 本计划默认 B1。

### 6.2 ridl-builder 侧（次要改动）

- `aggregate.rs`：不再生成 `ridl_modules_initialize.rs` / `ridl_bootstrap.rs`。
- 调用 ridl-tool 新的生成入口，写出：
  - `ridl_symbols.rs`
  - `ridl_context_ext.rs`
  - `ridl_bootstrap.rs`
  - `mquickjs_ridl_register.h`

### 6.3 mquickjs-ridl-glue / root build.rs（消费侧）

- copy 列表收敛为 3 个文件：
  - `ridl_symbols.rs`
  - `ridl_context_ext.rs`
  - `ridl_bootstrap.rs`

- 上层 include 点统一 include `ridl_bootstrap.rs`（或其导出的模块）。

---

## 7. 测试与验证

- 单测（建议补充）：
  - `ridl-tool`：生成 3 个文件后，断言文件存在；并对关键片段做少量字符串断言（例如 `CtxExt`、`initialize_modules`、`ensure_symbols`、`ridl_context_init`）。

- 集成：
  1) `cargo run -p ridl-builder -- aggregate ...` 后，检查 out_dir：
     - `ridl_symbols.rs` / `ridl_context_ext.rs` / `ridl_bootstrap.rs` 存在
     - 旧文件名不存在（验证“彻底消灭小文件”）
  2) `cargo test` 与 `cargo run -- tests` 通过。

---

## 8. 审阅结论（已确认）

- 产物数量：Rust 3 个文件（symbols/runtime_support/bootstrap）。
- 迁移策略：一次性切换；全量测试通过后直接不再生成旧文件。
- 实现路线：B（在 ridl-tool 中合并模板/生成器，直接生成大文件）。
