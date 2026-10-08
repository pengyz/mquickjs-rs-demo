# tests/

本目录存放 RIDL 集成测试语料（RIDL 测试模块与 `.js` 用例），不再承担 runner 兼容层职责：

- `global/`：GLOBAL 模式的 RIDL 测试模块（workspace 成员，如 `test_types`、`test_class`），
  每个模块的 `tests/*.js` 为对应 JS 集成用例。
- `module/`：module 模式的 JS 语料（配合 `ridl-modules/stdlib` 等模块使用）。
- `_diag/`：诊断输出目录，runner 收集时跳过所有 `_` 前缀目录。

JS 集成测试 runner 位于 `apps/demo`（`mquickjs-demo` bin）。运行命令：

```sh
cargo run -p mquickjs-demo -- tests          # 仅 tests/ 树
cargo run -p mquickjs-demo                   # 默认根：仓库根 tests/ + ridl-modules/
```

默认根按 workspace 根解析（不依赖 CWD），可在任意子目录运行。
