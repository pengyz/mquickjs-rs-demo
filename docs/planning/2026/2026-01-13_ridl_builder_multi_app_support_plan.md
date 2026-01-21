<!-- planning-meta
status: 未复核
tags: build, engine, ridl, tests
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `engine` `ridl` `tests`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# ridl-builder 多使用方/多应用支持方案（SoT=Cargo.toml，移除 profile）

日期：2026-01-13

## 1. 背景与目标

我们希望构建体系同时支持：

- **同一仓库内多个 app**（每个 app 注入自己的 RIDL module 列表）
- **多个外部使用方工程**（不同 repo/workspace 依赖 mquickjs-rs，并能指定自身的 RIDL module 列表）

并满足约束：

- **编译期注册**：不得依赖运行时 QuickJS C API 注册。
- **禁止硬编码模块名单**：模块发现必须通用可扩展。
- **C 侧聚合头必须复用 ridl-tool 模板生成**（避免 ridl-builder 拼字符串漂移）。
- **OUT_DIR 优先**：能放 OUT_DIR 的尽量不落稳定目录；但稳定目录仍用于外部工具消费（例如 mquickjs-build 输入的 register.h）。

本计划核心目标：

1. **彻底移除 profile / mquickjs.build.toml 机制**（不再作为选择 SoT）。
2. **唯一 SoT = app 的 Cargo.toml（direct deps 列表）**：RIDL module 集合完全由 app 的直接依赖决定（不递归处理模块的模块）。
3. ridl-builder 支持显式输入 **Cargo.toml 路径**：`--cargo-toml <path>`。
4. ridl-builder 引入显式构建意图：`--intent build|test`，用于选择 direct deps 集合：
   - build：仅 `[dependencies]`
   - test：`[dependencies] + [dev-dependencies]`
5. **强制迁移**聚合输出目录为按 app 隔离：`<target_dir>/ridl/apps/<app-id>/aggregate/`。
   - `target_dir` 默认来自 `cargo metadata` 的 `target_directory`
   - 可用环境变量 `MQUICKJS_RIDL_TARGET_DIR` 覆盖
6. 提供一个“标志性 crate”（建议：`mquickjs-app`），让 app **无需自写 build.rs**，仅通过 Cargo.toml 依赖即可完成 include/copy 接线。

> 说明：本阶段 build.rs **缺少聚合输出时直接报错**，提示先运行 ridl-builder prepare；后续如有必要再讨论自动触发。

## 2. SoT 定义：direct deps 即模块列表

RIDL module 判定规则（保持一致）：

- 对某个 crate：若其 `src/` 目录下存在至少一个 `*.ridl` 文件，则该 crate 被视为 RIDL module。

对某个 app（intent=build/test）：

- `RIDL modules(app, intent)` = app package 的 **direct deps** 中，所有满足上述判定规则的 crate 集合。
  - intent=build：direct `[dependencies]`
  - intent=test：direct `[dependencies] + [dev-dependencies]`

并且明确：

- 不递归：若某个 direct dep crate 自己依赖了包含 `*.ridl` 的 crate，不纳入模块集合。

因此关闭某些模块的方式就是 Cargo 原生机制：

- `optional = true` + `features` 控制 direct deps 是否启用
- `cfg(target_os=...)` 等平台条件（通过依赖声明生效）

不再引入 plan.json/menuconfig 作为第二套 SoT。

## 3. ridl-builder CLI 设计

### 3.1 参数

对 `aggregate` / `prepare`：

- `--cargo-toml <path>`：**必选**。目标 app 的 Cargo.toml（manifest-path）。
- `--app-id <string>`：可选。指定输出隔离用 app-id。

### 3.2 app-id 推导与规范化

当未提供 `--app-id`：

1. 通过 `cargo metadata --manifest-path <cargo-toml>` 读取对应 package 的 `name` 作为默认 app-id。
2. **规范化规则（确认）**：将 `name` 中的 `-` 替换为 `_`（其它字符策略后续再讨论）。

示例：`my-app` -> `my_app`。

## 4. 模块发现：使用 cargo metadata（闭包扫描）

实现算法：

1. 运行：`cargo metadata --format-version=1 --manifest-path <app/Cargo.toml>`
2. 解析 app package 对应 node，列出其 **direct deps**（一层）。
3. 根据 `--intent` 过滤 direct deps：
   - build：仅 normal deps
   - test：normal deps + dev deps
4. 对每个 direct dep package：若能定位到本地源码目录（workspace/path/git checkout），则扫描 `<crate_root>/src/*.ridl`。
5. 若 ridl_files 非空，则加入 `Module { crate_name, crate_dir, ridl_files }`。

保持原则：不基于“workspace 全扫描”，也不递归扫描闭包；只基于 app 的 direct deps。

## 5. 聚合输出：强制迁移与文件清单

### 5.1 输出目录

强制落点：

- `target/ridl/apps/<app-id>/aggregate/`

不再使用 `target/ridl/aggregate/`。

### 5.2 app-level 聚合产物（8 份）

1. `mquickjs_ridl_register.h`（C 侧聚合头；ridl-tool 模板生成）
2. `ridl_symbols.rs`（Rust 侧符号 keep-alive；ridl-tool 模板生成）
3. `ridl_slot_indices.rs`（singleton slots 索引；ridl-tool singleton_aggregate 生成）
4. `ridl_ctx_ext.rs`（ctx-ext；ridl-tool singleton_aggregate 生成）
5. `ridl_context_init.rs`（per-context init；ridl-tool singleton_aggregate 生成）
6. `ridl_modules_initialize.rs`（process-level modules init；ridl-builder 生成，后续可模板化）
7. `ridl_bootstrap.rs`（统一入口；ridl-builder 生成，保持 `crate::ridl_bootstrap::ridl_bootstrap::initialize()` 兼容）
8. `ridl-manifest.json`（审计快照；ridl-builder 生成）

## 6. 标志性 crate：mquickjs-app（避免 app 自写 build.rs）

### 6.1 目标

让应用只需在 Cargo.toml：

- 依赖 `mquickjs-app`
- 依赖（或通过 features 引入）所需 ridl modules

即可完成：

- include RIDL 聚合产物
- 提供稳定的 process/context 初始化 API

### 6.2 build.rs 行为（只 copy，不执行工具）

`mquickjs-app/build.rs`：

1. 定位 app 的 `Cargo.toml`（以 `CARGO_MANIFEST_DIR` 为基准）。
2. 推导 `app-id`（规则见 3.2）。
3. 计算聚合目录：`<workspace_root>/target/ridl/apps/<app-id>/aggregate/`。
4. 将以下文件拷贝到 `OUT_DIR`：
   - `ridl_symbols.rs`
   - `ridl_slot_indices.rs`
   - `ridl_ctx_ext.rs`
   - `ridl_context_init.rs`
   - `ridl_modules_initialize.rs`
   - `ridl_bootstrap.rs`
5. 若聚合目录或任一必需文件缺失：**直接 panic**，提示用户先运行：
   - `cargo run -p ridl-builder -- prepare --cargo-toml <app/Cargo.toml>`

### 6.3 对外 API（建议）

`mquickjs-app/src/lib.rs`：

- `include!(concat!(env!("OUT_DIR"), "/ridl_bootstrap.rs"))`
- `include!(concat!(env!("OUT_DIR"), "/ridl_context_init.rs"))`

并封装：

- `pub fn initialize_process()` -> 调用 ridl_bootstrap
- `pub fn initialize_context(ctx: *mut JSContext)` -> 调用 ridl_context_init

（细节以现有 mquickjs-rs/context API 为准）

## 7. 推荐工作流

两段式（避免 build.rs 中执行 cargo/工具）：

1. 预构建（生成聚合与 mquickjs 产物）：

```bash
cargo run -p ridl-builder -- prepare --cargo-toml path/to/app/Cargo.toml
```

2. 正常构建/测试：

```bash
cargo build
cargo test
```

## 8. 迁移步骤（强制迁移）

1. ridl-builder：
   - 移除 `--profile` 与 `mquickjs.build.toml` 解析
   - `aggregate/prepare` 改为必须提供 `--cargo-toml`
   - 输出目录改为 `target/ridl/apps/<app-id>/aggregate/`
   - 模块发现切到 `cargo metadata`
2. 引入 `mquickjs-app` crate：替代 app 自写 build.rs（或将现有 app build.rs 迁移到该 crate）。
3. 更新文档：明确 SoT=direct deps + features 开关。

## 9. 待讨论点（后续）

1. 是否允许 build.rs 自动运行 ridl-builder（当前不做）。
2. mquickjs-build 输出目录是否需要引入 `<app-id>` 分桶（当前不做）。
3. workspace_root 定位策略（跨 repo 时如何从 app 找到 target/，是否需要 env override）。

---

状态：计划草案（待评审）。
