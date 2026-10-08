---
name: ridl-builder-prepare-command
description: ridl-builder prepare 命令：构建工具、生成 RIDL 聚合、构建 QuickJS base/ridl 输出
type: reference
created: 2026-09-03
updated: 2026-10-08
sources: [README.md, ridl-builder/src/main.rs]
---

构建前准备命令：`cargo run -p ridl-builder -- prepare`（从仓库根运行）

**用途：**
1. 自动检测应用清单（发现规则见下）
2. 尝试使用 cargo unit-graph（nightly），否则 fallback
3. 生成 RIDL 聚合代码
4. 构建 QuickJS base 和 ridl 两套输出

**应用发现规则（2026-10-08 起，`default_cargo_toml_from_discovery`）：**
1. 显式 `--cargo-toml`（须绝对路径）优先；
2. 从 CWD 向上找最近 `mquickjs.ridl.toml`，取同目录 `Cargo.toml`
   （多应用约定：在 `apps/<name>/` 内运行即选该应用）；
3. 向上未命中时，取向上最近的 `mquickjs.build.toml` 的 `default` profile 的
   `app_manifest`（workspace 级 SoT，仓库根运行默认解析到 `apps/demo`）。

**何时使用：** 首次克隆仓库、修改 RIDL 定义、或遇到"Missing mquickjs build outputs"错误时运行。
