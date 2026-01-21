<!-- planning-meta
status: 未复核
tags: build, context-init, engine, ridl, tests
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `context-init` `engine` `ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# RIDL 编译流水线收敛方案（定稿草案）

日期：2026-01-13

## 目标

1. 定稿一条稳定的编译流水线：**禁止 build.rs 构建 Rust 工具**，避免 cargo 锁竞争/不确定性。
2. 明确项目分层与 crate 职责边界（Runtime / Codegen / Glue），让依赖关系可解释、可维护。
3. 明确 SoT（source of truth）与环境变量契约，支持多使用方/跨 repo。

## 背景约束（不可违背）

- mquickjs 的 C API 注册必须在**编译期**完成（不能运行期动态注册）。
- 禁止硬编码模块名单/白名单；模块发现必须是通用、可扩展机制。
- Rust 实现 RIDL module 会引入“符号保活/链接裁剪”风险，必须通过**聚合引用点**或等价手段保证模块不会被裁剪。

## 分层与 crate 职责（单一职责 + 依赖边界）

### 1) Runtime 层（运行时库）

- `deps/mquickjs-sys`
  - QuickJS C 绑定、底层 FFI。
  - 不关心 RIDL 模块选择与聚合。

- `deps/mquickjs-rs`
  - Rust 安全封装（Context/ProcessHandle 等）。
  - 提供对“聚合产物”的消费入口（例如 `ridl_bootstrap!()` 宏），但**不负责**模块发现/聚合。

### 2) Codegen 层（生成工具；严禁在 build.rs 里构建/运行）

- `deps/ridl-tool`
  - 纯 codegen/template/render 库（可被工具/构建器复用）。

- `ridl-builder`
  - orchestrator：读取 `cargo metadata`，按 SoT 选择模块集合，调用 ridl-tool 生成聚合产物。
  - 产物输出到 `<target_dir>/ridl/apps/<app-id>/aggregate/`。
  - 该工具只能通过显式命令运行（prepare/aggregate），**不允许**在任意 build.rs 内隐式运行。

### 3) Glue 层（接线层；build.rs 只做 copy，不做构建/生成）

- `deps/mquickjs-ridl-glue`
  - `build.rs`：从 `<target_dir>/ridl/apps/<app-id>/aggregate/` 拷贝 `*.rs` 到 `OUT_DIR`。
  - `lib.rs`：薄封装，转调 `mquickjs-rs` 的初始化入口（process/context）。

## SoT（模块选择的唯一来源）

- SoT = “root crate 的 Cargo.toml 的 direct deps”（按 intent 区分 build/test）。
- ridl module 判定：仅当依赖 crate 的 `src/` 下存在至少一个 `*.ridl` 文件，才被视为 ridl module。
- 不递归处理模块依赖中的模块（只看 direct deps）。

### 重要注意：RIDL 模块必须是 `[dependencies]`

- 对 intent=build：ridl-builder 只读取 app/root crate 的 **direct `[dependencies]`** 作为模块候选集；
  **不会**从 `[build-dependencies]` 中选模块。
- 如果误把 RIDL 模块（例如 `stdlib` / `ridl_module_*`）声明在 `[build-dependencies]`：
  - ridl-builder 聚合时会得到空模块列表，导致 `mquickjs_ridl_register.h` 不包含任何扩展。
  - mquickjs-build 生成的 `mqjs_ridl_stdlib.h` 也不会注入 `JS_RIDL_*` 表项。
  - 最终表现为 JS 侧 `console` / `default_*` / `strict_*` 等符号为 `undefined`。

实践建议：
- 仅把“build-helper glue（负责 copy 聚合产物到 OUT_DIR）”放在 `[build-dependencies]`。
- 所有 RIDL 模块 crate 必须放在 `[dependencies]`（可配合 `optional + features` 做开关）。

## app-id（实际上是 root-id）

- `app-id` 仅用于隔离输出目录；语义上等价于“root crate id”。
- 规范化规则：将所有非 `[A-Za-z0-9_]` 字符替换为 `_`。

## 目录与产物

- 输出根目录：`<target_dir>/ridl/apps/<app-id>/aggregate/`
- `target_dir` 默认来自 `cargo metadata.target_directory`，可用 `MQUICKJS_RIDL_TARGET_DIR` 覆盖。

产物（当前列表）：
- `ridl-manifest.json`
- `mquickjs_ridl_register.h`
- `ridl_symbols.rs`
- `ridl_slot_indices.rs`
- `ridl_ctx_ext.rs`
- `ridl_context_init.rs`
- `ridl_modules_initialize.rs`
- `ridl_bootstrap.rs`

## 一次性配置文件（推荐）

为避免每次 `cargo build/test` 都需要手动设置环境变量，引入一次性配置文件：

- 文件名：`mquickjs.ridl.toml`
- 放置位置：需要聚合 RIDL 的“root crate”目录（即其 `Cargo.toml` 同级）。
- 读取方式：`mquickjs-ridl-glue` 的 build.rs 从 `CARGO_MANIFEST_DIR` 开始向上逐级搜索该文件，
  采用“**就近命中（nearest wins）**”。

### 配置格式（最小集合）

```toml
version = 1

# 可选：覆盖输出目录隔离 id；默认使用 root package.name 规范化
# app_id = "my_root"

# 可选：覆盖 target_dir（跨 repo/多使用方）
# target_dir = "/abs/path/to/target"

# 可选：默认 intent（build/test），仅用于工具默认值；build.rs 本身只负责 copy
# intent = "build"
```

### 误命中风险与规避

- 风险：如果父目录（例如 monorepo 根）也存在同名文件，可能被误命中。
- 规避：
  1) 采用 nearest-wins（从当前 crate 往上，先找到哪个用哪个）
  2) 文件内必须包含 `version = 1`（作为最小 "magic"），否则视为无效
  3) 必要时仍可用环境变量强制覆盖（用于 CI/特殊布局）

## 环境变量契约

### ridl-builder

- `MQUICKJS_RIDL_TARGET_DIR`：覆盖 `cargo metadata.target_directory`。

### mquickjs-ridl-glue

- `MQUICKJS_RIDL_TARGET_DIR`：同上。
- `MQUICKJS_RIDL_APP_ID`：覆盖 app-id（默认：root package.name 规范化）。
- `MQUICKJS_RIDL_CARGO_TOML`：强制指定 root Cargo.toml（优先级最高；用于 CI/特殊布局；默认可由 `mquickjs.ridl.toml` 自动发现）。

> 关键点：glue crate 的 build.rs 不能可靠地“自动推断谁是 root crate”。
> 因此默认通过 `mquickjs.ridl.toml` 的向上搜索（nearest-wins）来确定 root。
> 对于 CI/特殊布局，可用环境变量强制覆盖。

## 标准工作流（推荐）

1. 生成/更新聚合产物（显式执行，避免 build.rs 锁竞争）：

```bash
cargo run -p ridl-builder -- aggregate --cargo-toml /abs/path/to/root/Cargo.toml --intent build
```

2. 构建 root crate（glue crate build.rs 只负责拷贝）：

```bash
cargo build -p <root-crate>
```

3. 测试 intent=test 时的聚合：

```bash
cargo run -p ridl-builder -- aggregate --cargo-toml /abs/path/to/root/Cargo.toml --intent test
```

## 依赖关系图（简化）

```
        +-------------------+
        |    ridl-builder   |  (tool; explicit run)
        +-------------------+
                 |
                 | generates
                 v
<target_dir>/ridl/apps/<app-id>/aggregate/*
                 ^
                 | copies at build time
        +-------------------+
        | mquickjs-ridl-glue|  (build.rs only copy)
        +-------------------+
                 |
                 v
        +-------------------+
        |    root crate     |  (SoT; selects modules in Cargo.toml)
        +-------------------+
                 |
                 v
        +-------------------+
        |    mquickjs-rs    |  (runtime)
        +-------------------+
                 |
                 v
        +-------------------+
        |   mquickjs-sys    |  (ffi)
        +-------------------+
```

## 待定/后续优化（不在本次定稿强制）

- 聚合产物文件数量可考虑合并（减少 ctx_ext/init 多文件），但这属于“减法优化”，不影响流水线边界。
- mquickjs-build 的输出目录未来可按 app-id 隔离（目前仍使用 framework/profile 目录）。
