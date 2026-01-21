<!-- planning-meta
status: 未复核
tags: ridl, tests
replaced_by:
- docs/ridl/overview.md
-->

> 状态：**未复核**（`ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：引入 api.rs 层，重构 RIDL 生成物依赖关系（api -> impl -> glue）

日期：2026-01-12

## 背景

当前 ridl-tool 会在模块 crate 的 `OUT_DIR` 生成 `<module>_impl.rs`（包含 trait + `todo!()` stub）。但我们已经明确：

- 模块的 `impl.rs` 必须由用户手写（可在 `src/` 或 crate 根文件，参考 stdlib 的 `stdlib_impl.rs`）。
- glue.rs 作为转换层，依赖用户手写实现。
- 为避免依赖反转，用户实现层只能依赖“纯 Rust 的类型/trait 声明”，不应依赖 glue。

因此需要把“生成的 trait/类型声明”从当前的 `*_impl.rs` 中抽离到新的 `*_api.rs`，并停止生成 `*_impl.rs`。

## 目标

1) **生成物分层清晰**：在每个 RIDL 模块 crate 内形成稳定的依赖方向：
   - `api.rs`（生成） -> `impl.rs`（手写） -> `glue.rs`（生成）
2) **停止生成 `OUT_DIR/<module>_impl.rs`**，避免产生误导性 stub。
3) **保留 trait 生成能力**：singleton/class 的 trait/interface 声明由 generator 生成，但位置应在 `OUT_DIR/<module>_api.rs`。
4) **为未来复杂类型演进留出位置**：struct/enum/type alias 等声明进入 `api.rs`。
5) **不引入硬编码**：聚合/注册/初始化仍保持通用机制。
6) 测试覆盖：`cargo test` + `cargo run -- tests` 均通过。

## 现状确认（需要修正的点）

- `deps/ridl-tool/src/generator/mod.rs` 当前会写出：`output_path.join(format!("{}_impl.rs", module_name))`
- 模块 crate（例如 stdlib）当前 include：
  - `OUT_DIR/stdlib_glue.rs`
  - `OUT_DIR/stdlib_symbols.rs`
  - `OUT_DIR/stdlib_impl.rs`（这就是我们要移除/替换的）

## 方案概述

### 文件与模块布局（每个 RIDL 模块 crate）

建议在模块 crate 的 `src/lib.rs` 内统一组织生成物路径，避免用户到处 `include!(OUT_DIR/...)` 造成路径不稳定：

- `mod generated { pub mod api { include!(.../<module>_api.rs) } pub mod glue { include!(.../<module>_glue.rs) } pub mod symbols { include!(.../<module>_symbols.rs) } }`
- 对外导出：
  - `pub use generated::glue::*;`（C ABI wrapper）
  - `pub use generated::api::*;`（trait/类型）

用户手写实现可按 stdlib 的模式：
- `#[path = "../stdlib_impl.rs"] mod stdlib_impl;`
- `pub mod impls { ... }` 中导出实现函数与构造器

### generator 侧输出

- 新增：`<module>_api.rs`（纯 Rust：trait/类型声明）
- 保留：`<module>_glue.rs`、`<module>_symbols.rs`、`ridl_module_api.rs`
- 移除：`<module>_impl.rs`

### glue 侧依赖

- glue 内引用 trait 路径：`crate::generated::api::<Trait>`
- glue 调用实现入口：`crate::impls::*`（由用户手写导出）

## 分阶段落地步骤

### Phase 0：文档与约束确认（你已确认）

- trait/复杂类型声明属于 api 层；impl 手写；glue 负责转换与调用。

### Phase 1：ridl-tool 生成物重构（最小变更）

1) 在 ridl-tool 增加 `rust_api.rs.j2` 模板：
   - 生成 singleton/class 的 trait/interface
   - 先不生成 free function 的 stub（必要时仅声明签名类型）
   - 未来复杂类型声明入口预留（先留空/注释）

2) 修改 `deps/ridl-tool/src/generator/mod.rs`：
   - 停止渲染/写入 `RustImplTemplate`（即不写 `<module>_impl.rs`）
   - 新增渲染/写入 `RustApiTemplate`（写 `<module>_api.rs`）

3) 更新 `deps/ridl-tool/templates/rust_glue.rs.j2`：
   - 引用 trait 路径从 `crate::generated::impls::Xxx` 改为 `crate::generated::api::Xxx`
   - 保持现有转换与错误处理行为不变

验收：
- `cargo test` 通过（含 JS smoke）

### Phase 2：模块 crate 组织方式调整

对 `ridl-modules/stdlib`、`ridl-modules/ridl_module_demo_default`、`ridl-modules/ridl_module_demo_strict`：

1) 在 `src/lib.rs` 的 `mod generated { ... }` 内：
   - 替换 `pub mod impls { include!(.../<module>_impl.rs) }` 为 `pub mod api { include!(.../<module>_api.rs) }`
2) 更新 `pub mod impls { ... }` re-export：
   - `pub use crate::generated::api::ConsoleSingleton;`（原来可能是 `generated::impls`）

验收：
- `cargo test` 通过
- `cargo run -- tests` 通过

### Phase 3：清理与回归

1) 删除 ridl-tool 内 `rust_impl.rs.j2` 及相关代码路径（或保留但不再使用，视维护策略）
2) 搜索仓库中对 `OUT_DIR/*_impl.rs` 的 include，并全部迁移到 `*_api.rs`
3) 更新 `docs/ridl/` 中相关设计文档，确保与实现一致

验收：
- 全仓库无 `*_impl.rs` 生成物依赖（除了历史 build 产物）

## 风险与注意事项

- Rust 模块路径稳定性：推荐统一通过 `crate::generated::api` 暴露 trait，避免用户在不同位置 include 导致路径漂移。
- complex types：短期依旧走 `any(JSValue)`，只把声明位置放到 api；具体转换仍在 glue，后续再引入 wrapper。
- 不要在 api 层引入 JSValue/JSContext 的转换逻辑（避免 impl 层被污染）。

## 验收清单

- ridl-tool：不再生成 `<module>_impl.rs`，改为生成 `<module>_api.rs`
- 所有 RIDL 模块 crate：`generated::api` 存在且对外可用；`generated::impls` 不再使用
- JS 集成用例：`cargo run -- tests` 全通过
- `cargo test` 通过
