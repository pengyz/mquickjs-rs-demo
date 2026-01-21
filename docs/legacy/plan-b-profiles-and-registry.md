# Plan B：Profiles + App Manifest Registry（历史/部分过时）

> 历史/部分过时：本文描述“Plan B”的方案设计与未来规划，不保证与当前实现一致。
>
> 现行口径请以以下文档为准：
> - `docs/build/pipeline.md`
> - `docs/architecture/overview.md`


> 本文档是当前仓库的**权威方案说明**，用于约束后续实现与重构。
> 
> 目标：在 **mquickjs 仅支持“编译时注册标准库扩展”** 的前提下，实现可维护的 RIDL 模块选择、聚合与构建分桶（profiles）。

## 0. 背景与约束

- QuickJS/mquickjs 的扩展注册依赖 C 侧 stdlib 扩展表（例如 `mqjs_stdlib_template.c`）在**编译期**包含扩展定义。
- 因此 RIDL 扩展无法在运行时以 Rust 方式动态注册；必须在构建 C 静态库时决定并编译进 `libmquickjs.a`。

## 1. 术语

- **RIDL module**：一个 Rust crate，满足：其 `src/` 下至少包含一个 `*.ridl` 文件。
- **registry source / app manifest**：某个 `Cargo.toml`，作为 RIDL 模块列表的 SoT（source of truth）。
  - RIDL 模块列表来源于该 manifest 的依赖图（path deps），并按“是否包含 `src/*.ridl`”过滤。
- **profile**：一套构建形态（如 `core`/`framework`/`tooling`/`tests`），用于
  - 选择 registry source（app manifest）
  - 分桶隔离构建产物
  - 未来扩展 C 编译选项、开关等

## 2. 方案核心

### 2.1 SoT：以最终 App 的 Cargo 依赖图作为注册源

- 不再要求存在专门的 `ridl-modules/registry` 作为 SoT（当前 SoT 为 App manifest：根 `Cargo.toml` 的 `[dependencies]`）。
- 最终 App（例如仓库根 `mquickjs-demo` 的 `Cargo.toml`）作为 registry source：
  - App 显式以 path dependency 依赖所需 RIDL modules（如 `ridl-modules/ridl_module_demo_default` / `ridl-modules/ridl_module_demo_strict`）。
  - 构建时根据该依赖图解析得到“需要注册的 RIDL modules”。

这能自然支持复杂工程：
- A、B 都依赖 `mquickjs-sys/mquickjs-rs`；真正的“最终注册集合”由最终二进制（App）决定。
- A/B 作为库不应偷偷决定最终 JS 扩展集合。

### 2.2 Profiles：一组命名构建形态（新增 profile = 新增 registry source）

- Profiles 解决的问题：在同一个 workspace 中产出不同构建形态，并隔离产物。
- **新增 profile 必须对应一个 registry source（app manifest）**：
  - 例如未来 `core`/`framework` 的 RIDL 集合不同，推荐使用不同的 app manifest（如 `apps/core/Cargo.toml`、`apps/framework/Cargo.toml`）。
- 目前允许多个 profile 暂时指向同一个 app manifest（用于验证 profile 机制本身），但这只是一种过渡状态。

## 3. 构建链路（单点构建：mquickjs-sys）

### 3.1 mquickjs-sys 的职责

`deps/mquickjs-sys/build.rs` 是 C 产物与 bindings 的单点构建入口：

1) 读取 `mquickjs.build.toml`：确定当前 profile 与其 `app_manifest`
2) 调用 `ridl-tool resolve --manifest-path <app_manifest>`：
   - 解析依赖图
   - 仅保留 path deps 且其 `<dep>/src` 含 `*.ridl` 的 crate
   - 生成 `ridl-manifest.json`
3) 由 `ridl-builder prepare` 调用 ridl-tool 进行聚合输出（3 个 Rust 大文件 + 1 个 C 头）：
   - 生成 per-module 产物（Rust glue、symbols 等）
   - 生成聚合头 `mquickjs_ridl_register.h`（用于 C 侧编译期注入 stdlib 扩展）
4) 调用 `mquickjs-build`：
   - 编译 C 静态库 `libmquickjs.a`
   - 将 `mquickjs_ridl_register.h` 注入到 stdlib 扩展编译路径中
   - 输出 include/lib/json 到分桶目录
5) bindgen：
   - 以 Rust 2024 生成 bindings（`rust_edition(Edition2024)`）

### 3.2 产物分桶

- C 构建输出统一放在：
  - `target/mquickjs-build/<profile>/<target>/<debug|release>/...`

## 4. mquickjs.build.toml 的发现规则（无硬编码 + 可覆盖）

mquickjs-sys 不应假设仓库布局。

### 默认发现
- 从 `CARGO_MANIFEST_DIR`（mquickjs-sys 自身 crate 目录）向上查找 workspace root：
  - 找到包含 `[workspace]` 的 `Cargo.toml` 所在目录
- 读取 `<workspace_root>/mquickjs.build.toml`

### 环境变量覆盖（推荐用于复杂集成）
- 若设置 `MQUICKJS_BUILD_TOML`，则优先使用该路径。
- 注意：此 env 必须由“外部环境”注入（shell / `.cargo/config.toml` / CI env）。
  - `cargo:rustc-env` 不会传播到其他 crate 的 build.rs。

## 5. bindgen / Rust Edition

- 项目统一 edition 2024。
- bindgen 需使用支持 Rust 2024 输出的版本，并在生成时设置：
  - `.rust_edition(bindgen::RustEdition::Edition2024)`

## 6. 目前状态与下一步

- 目前 profiles 可保持三份（framework/tooling/tests）指向同一个 app manifest（用于验证机制）。
- 下一阶段：引入 `apps/<profile>/Cargo.toml` 作为不同 registry source，以验证 core/framework 子集差异。
