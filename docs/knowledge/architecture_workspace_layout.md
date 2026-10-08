---
name: workspace-layout
description: 工作区目录职责约定：虚拟清单、apps/ 放应用、deps/ 混放产品与 vendored 引擎
type: architecture
created: 2026-10-08
sources: [docs/planning/2026/2026-09-15-sdk-repositioning.md, README.md]
---

**布局（2026-10-08 起）**：根 `Cargo.toml` 是虚拟清单（仅 `[workspace]` +
`resolver = "2"`，根不拥有包）。

| 路径 | 职责 |
|------|------|
| `deps/mquickjs-rs`、`deps/ridl-tool`、`deps/mquickjs-ridl-glue`、`deps/mquickjs-build` | 产品 crate（虽在 deps/ 下，是本仓库产出物） |
| `deps/mquickjs`、`deps/mquickjs-sys` | vendored C 引擎与 FFI 绑定（真正的依赖） |
| `apps/demo`、`apps/test_app` | 应用；demo 即原根包（包名 `mquickjs-demo` 保留），JS 测试 runner 在其 main.rs |
| `tests/` | 仅 RIDL 一致性测试模块与 JS 语料（`tests/*.rs` 集成测试已随包迁至 `apps/demo/tests/`） |
| `ridl-modules/stdlib` | RIDL 标准库（自举：stdlib 即 RIDL 模块） |

**关键约定**：
- 命令一律从仓库根运行：`cargo run -p mquickjs-demo`（无参=全量）、
  `cargo run -p ridl-builder -- prepare`（多应用见
  [[ridl-builder-prepare-command]] 的发现规则）
- `deps/` 名不副实是**已知且接受**的：改名波及全部路径依赖/build.rs/hooks/文档，
  零功能收益，以本条目说明替代
- 集成测试进程 CWD = 包根（apps/demo），访问 `target/`、`deps/`、JS 语料须经
  `mquickjs_demo::test_runner::workspace_root()`（锚点：`mquickjs.build.toml`）

相关：[[decision-sdk-repositioning]]
