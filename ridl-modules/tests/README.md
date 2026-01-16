# ridl-modules/tests/

本目录存放 **RIDL 框架测试模块**（不是功能模块）。

## 目录约定

- 功能模块：放在 `ridl-modules/stdlib/`（以及未来可能出现的其它功能模块目录）。
- 测试模块：统一放在 `ridl-modules/tests/`。

当前仅启用：
- `global_mode/`：覆盖 global 注册模式下的 RIDL 语法与 glue 行为。

> 说明：`module_mode` 当前尚未支持；后续支持后再新增 `module_mode/` 并按相同方式组织。

## 每个测试模块的结构

每个测试模块应是一个独立 crate（用于被 ridl-builder 聚合与生成 glue），并遵循：

- `src/*.ridl`：该语法域的 RIDL 输入（尽量最小化）。
- `src/lib.rs` + `src/*_impl.rs`：实现生成的 Rust trait（命名/签名以生成产物为准）。
- `tests/*.js`：端到端 JS 集成测试用例。
- `README.md`：如遇暂缓/绕开项，用 README 记录原因与后续修复点。

## 如何新增用例

1. 在 `ridl-modules/tests/global_mode/<domain>/` 新建测试模块目录。
2. 编写 `src/*.ridl` 与 Rust impl。
3. 在同目录 `tests/` 添加 JS 用例。
4. 运行验证：
   - `cargo run -p ridl-builder -- prepare`
   - `cargo run -- tests`

## 注意事项（当前限制）

- v1 glue 的类型支持仍有限（nullable/optional/union 等可能触发 compile_error），相关缺口见：
  - `doc/planning/2026-01-16_test_reorg_followups.md`
- 生成命名规则目前非 Rust idiomatic（全小写/压平），实现需跟随生成产物。
