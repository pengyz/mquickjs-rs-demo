# 构建流水线（现行口径）

本文档描述本仓库的构建链路：从 app manifest 决定 RIDL modules，到生成聚合产物，再到 mquickjs C 侧编译期注册。

## 1. 核心约束

- mquickjs：QuickJS C API 注册必须编译期完成，不能运行期动态注册。
- RIDL module 判定：依赖路径 `src/` 下至少 1 个 `*.ridl` 才算。

## 2. 产物与阶段

1) 解析依赖图（registry source / app manifest）
- 输入：`--cargo-toml` 指定的 app `Cargo.toml`
- 输出：`ridl-manifest.json`（模块选择快照）

2) ridl-tool 生成聚合产物（app OUT_DIR）
- `mquickjs_ridl_register.h`
- `ridl_symbols.rs`
- `ridl_context_ext.rs`
- `ridl_bootstrap.rs`

3) mquickjs-build 编译 C 静态库
- 将 `mquickjs_ridl_register.h` 注入 stdlib 编译路径
- 输出 `libmquickjs.a` 与 headers/bindings

4) app 编译/链接
- Rust 侧 include `ridl_context_ext.rs` 并在 Context 创建后调用 `ridl_context_init(ctx)`

## 3. 工具与入口

- `ridl-builder`：生成/准备聚合输出与构建输入
- `deps/mquickjs-ridl-glue`：将聚合输出拷贝到各 crate 的 OUT_DIR
- app：`src/context.rs` / `src/ridl_context_init.rs`
