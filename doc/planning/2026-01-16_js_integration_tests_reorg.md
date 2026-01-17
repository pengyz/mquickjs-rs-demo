# JS 集成测试重组方案（2026-01-16）

## 目标

- 将所有 JS 集成用例从 repo 根 `tests/` 迁移到对应 RIDL module 内部（模块自带 tests 目录）。
- 按测试维度重组为若干“测试模块”（framework-level），位于：`ridl-modules/tests/`。
- 功能模块（例如 `ridl-modules/stdlib`）也可以拥有自己的模块内 JS 用例，用于验证业务/功能正确性。

## 约束与现状

- 运行入口：`cargo run -- <path>`，当前 CI/约定用法：`cargo run -- tests`。
- 运行器实现：`src/test_runner.rs::collect_js_files()` 支持传入目录并递归收集 `*.js`。
  - 目前传入 `tests/` 时会跑根目录下所有 JS 用例。
- RIDL 注册：编译期注册（`mquickjs_rs::ridl_bootstrap!()`）。
  - 本阶段先保持“全量一起跑”：即构建时包含功能模块 + framework 测试模块。
  - 后续如需拆分 SoT / 多测试应用，再单独规划。

## 目录结构（目标）

```
ridl-modules/
  stdlib/
    src/*.ridl
    tests/*.js              # 功能模块测试（验证 stdlib 行为）

  tests/
    test_fn/
      src/*.ridl
      tests/*.js            # 函数语法维度（含 nullable/union/varargs/strict 等）

    test_var/
      src/*.ridl
      tests/*.js            # 变量/初始化维度（含 string literal、js-only fields）

    test_class/
      src/*.ridl
      tests/*.js            # class 维度（构造、方法、属性、proto 行为、错误）

    test_singleton/
      src/*.ridl
      tests/*.js            # singleton 维度

    test_import_module/
      src/*.ridl
      tests/*.js            # module/import/using 维度
```

## 用例迁移映射（第一批）

> 现有根 tests/ 下 9 个用例迁移规则：

- console 相关：
  - `tests/smoke_console_enabled.js` -> `ridl-modules/stdlib/tests/console_enabled.js`
  - `tests/smoke_console_log.js` -> `ridl-modules/stdlib/tests/console_log.js`
  - `tests/smoke_console_log_varargs.js` -> `ridl-modules/stdlib/tests/console_log_varargs.js`

- class 相关（现有 `tests/smoke_test_class_*`）：
  - 迁移到 framework 模块：`ridl-modules/tests/test_class/tests/`
    - `smoke_test_class_basic.js` -> `basic.js`
    - `smoke_test_class_proto.js` -> `proto.js`
    - `smoke_test_class_errors.js` -> `errors.js`

- ridl demo / glue / varargs：
  - 暂时归入 `test_fn`（后续如需可拆 `test_glue` / `test_variadic`）
    - `tests/smoke_ridl_strict_demo.js` -> `ridl-modules/tests/test_fn/tests/strict_demo.js`
    - `tests/smoke_ridl_v1_glue.js` -> `ridl-modules/tests/test_fn/tests/v1_glue.js`
    - `tests/smoke_ridl_varargs.js` -> `ridl-modules/tests/test_fn/tests/varargs.js`

## 运行方式（本阶段）

- 仍然使用 `cargo run -- tests` 作为入口，但把 `tests/` 目录调整为一个“聚合目录”，只负责承载所有 JS 用例（通过目录/软链接/子目录）。

为了满足“所有 JS 用例都迁移到模块目录”，本阶段计划将 repo 根 `tests/` 改为：

- runner 未传参时默认扫描 `tests/` 与 `ridl-modules/`（临时写死），不再依赖 `tests/ridl-modules` 软链接。

然后 `cargo run -- tests` 会递归扫描 `tests/` 与 `ridl-modules/`。

> 注意：这是过渡方案：先把框架级用例收敛到 `tests/global/**`，并让 runner 同时扫描两棵树；后续再补选择/过滤规则。

## 分组维度说明

- `test_fn`：函数签名/参数/返回、nullable/union|null 规范化、any 允许 null、varargs/strict 相关。
- `test_var`：变量声明与初始化、string literal escapes（\\n/\\t/\\r/\\\"/\\\\）、非法 escape 报错。
- `test_class`：class 定义、constructor、method/property、proto 相关。
- `test_singleton`：singleton 定义/注册/访问。
- `test_import_module`：module/version、import/from、using、错误定位。

## 后续演进（非本阶段）

- 支持按 suite 拆分（功能模块 vs framework 测试模块），可能需要 SoT 切换或引入新的测试应用。
- 支持 runner 原生扫描 `ridl-modules/**/tests/**/*.js`，彻底移除根 `tests/` 过渡入口。

## 完成标准

- 根 `tests/*.js` 清空（迁移完成）。
- 新目录结构落地，JS 用例全部位于 module 的 `tests/` 中。
- `cargo run -- tests` 全量通过。
- `cargo test` 全量通过。
