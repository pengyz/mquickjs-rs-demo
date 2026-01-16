# tests/TODO

> 目标：把当前 JS 集成测试暴露的问题逐项修复，并在每一步保持 `cargo run -- tests` 全绿。

## P0（阻塞型 / 影响面大）

### 1. v1 glue：补齐 optional/nullable/union 支持
- 现象：遇到 `T?`、`(A | B | null)?`、optional/union 路径时生成 `compile_error!("v1 glue: unsupported parameter type")`。
- 影响：无法恢复/新增 nullable 端到端用例（global_mode/test_types 等）。
- 产出：
  - glue 支持 `int?`/`string?`/`bool?` 的参数与返回
  - glue 支持 `A | B | null` 与整体可空 `(A|B|null)?` 的 runtime check/转换
  - 恢复相关 JS 断言（不再只是 smoke/no-op）。

### 2. ridl-builder 聚合链路：支持 singleton var/proto var 成员
- 现象：singleton 内出现 `var ... = literal;` 或 `proto var ... = literal;` 时聚合报错 `expected singleton_member`。
- 影响：JS-only fields 的端到端验证无法落地（global_mode/test_js_fields、test_literals 的部分用例）。
- 产出：
  - 聚合可解析 var_member
  - 生成与注册链路一致
  - 恢复 test_js_fields/test_literals 的断言与覆盖。

### 3. ridl-builder 聚合链路：enum/struct/msgpack struct 端到端
- 现象：聚合链路对包含 enum/struct 的 RIDL 文件解析不一致，导致 test_struct_enum 暂缓。
- 产出：
  - `ridl-modules/tests/global_mode/test_struct_enum` 纳入 app 依赖
  - 新增/恢复 JS 用例并通过。

### 4. class glue：修复生成代码错误与 constructor hook 约定
- 现象：生成的 glue 出现 `_argc/_argv` 与 `argc/argv` 使用不一致的编译错误；并且要求 `crate::impls::<class>_constructor()` 等 hook。
- 产出：
  - glue 生成可编译
  - 明确并实现 class ctor/proto state hook 约定
  - 恢复/新增 class 端到端用例并纳入全量测试。

## P1（质量/一致性）

### 5. 生成命名规则：Rust idiomatic 命名（破坏性变更）
- 现象：`TestDiagnostics` -> `TestdiagnosticsSingleton`，`getName` -> `getname` 等。
- 影响：手写 impl 与 API 体验差；也容易造成对齐成本。
- 产出：
  - 统一命名策略（trait/class/方法 snake_case + CamelCase）
  - 提供迁移路径/兼容策略（必要时）。

### 6. runner：去掉 tests/ridl-modules 软链接兼容层
- 现状：runner 通过 `tests/ridl-modules -> ../ridl-modules` 过渡来发现模块内 tests。
- 产出：runner 原生扫描仓库根 `ridl-modules/**/tests/**/*.js`。

## 工作方式

- 每完成一项：
  1) 补齐/恢复对应 JS 断言
  2) 运行 `cargo run -p ridl-builder -- prepare`
  3) 运行 `cargo run -- tests`
  4) 必要时 `cargo test`
- 每项做完再提交一次（保持提交小且可回溯）。
