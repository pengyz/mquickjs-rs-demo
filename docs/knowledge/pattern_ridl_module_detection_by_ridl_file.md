---
name: ridl-module-detection-by-ridl-file
description: 仅当 crate 的 src/ 目录包含 *.ridl 文件时才作为 RIDL 模块处理
type: pattern
created: 2026-09-03
sources: [AGENTS.md]
---

Treat a crate as a RIDL module **only if** the dependency path's `src/` directory contains at least one `*.ridl` file。

**为什么：** 避免将普通 Rust crate 误判为 RIDL 模块导致错误的聚合和注册。RIDL 模块的识别标准是"有 RIDL 定义文件"，而非 Cargo.toml 中的字段或命名约定。这确保了模块检测的可靠性，不会因为命名相似或依赖关系而误触发 RIDL 处理流程。

**何时使用：** 在 ridl-builder 或其他工具中实现 RIDL 模块发现逻辑时，必须检查 `src/*.ridl` 文件存在性。不要依赖 crate name、Cargo.toml 字段或目录结构作为判断依据。
