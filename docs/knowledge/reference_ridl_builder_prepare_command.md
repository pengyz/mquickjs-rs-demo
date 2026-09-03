---
name: ridl-builder-prepare-command
description: ridl-builder prepare 命令：构建工具、生成 RIDL 聚合、构建 QuickJS base/ridl 输出
type: reference
created: 2026-09-03
sources: [README.md]
---

构建前准备命令：`cargo run -p ridl-builder -- prepare`

**用途：**
1. 自动检测 Cargo.toml（从 mquickjs.ridl.toml 所在目录）
2. 尝试使用 cargo unit-graph（nightly），否则 fallback
3. 生成 RIDL 聚合代码
4. 构建 QuickJS base 和 ridl 两套输出

**何时使用：** 首次克隆仓库、修改 RIDL 定义、或遇到"Missing mquickjs build outputs"错误时运行。
