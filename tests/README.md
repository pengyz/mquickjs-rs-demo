# tests/

本目录作为测试入口/兼容层存在：

- `js_smoke.rs`：Rust 层的 smoke test，会通过 `cargo run -- tests` 跑一轮 JS 集成用例。
- `ridl-modules`：指向仓库根 `ridl-modules/` 的软链接，用于让 runner 继续在 `tests/` 下发现模块内的 `tests/*.js`。

长期目标：runner 原生支持从仓库根扫描 `ridl-modules/**/tests/**/*.js` 后，可移除该软链接兼容层。
