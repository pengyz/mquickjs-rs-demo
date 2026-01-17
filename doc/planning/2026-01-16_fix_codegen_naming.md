# 修复生成命名规则（破坏性变更，一次性切换）

日期：2026-01-16

## 背景与目标

当前 ridl-tool 生成 Rust API 的命名规则存在“全小写化/压平”问题，例如：

- `TestDiagnostics` -> `TestdiagnosticsSingleton`
- `TestFn` -> `TestfnSingleton`
- `getName` -> `getname`
- ctor 符号：`ridl_create_testdiagnostics_singleton`（无词边界）

这导致：
- 手写 impl 对齐成本高，容易因大小写/下划线不一致而失败
- API 不符合 Rust idiomatic 风格，影响测试与模块开发体验

本计划目标：
- **一次性破坏性**切换为 Rust idiomatic 命名（不做兼容层）。
- 保证 `cargo run -p ridl-builder -- prepare`、`cargo run -- tests`、`cargo test` 全绿。

## 统一命名规范（SoT）

### 1) Rust 类型（trait / struct / enum 等）
- 采用 **UpperCamelCase**（PascalCase）。
- 词边界来源：
  - 原始 RIDL 标识符的 CamelCase 边界（含 lowerCamel 与 UpperCamel）
  - 下划线 `_` 视为词边界
  - 连续数字视为一个 token
- 缩写（连续大写）保持为 token（例如 `URLValue`、`JSValue`）。

示例：
- `TestDiagnostics` -> `TestDiagnosticsSingleton`
- `test_diagnostics` -> `TestDiagnosticsSingleton`
- `JSFields` -> `JSFieldsSingleton`

### 2) Rust 方法名
- 采用 **snake_case**。
- 词边界同上（CamelCase/下划线/数字）。

示例：
- `getName` -> `get_name`
- `echoAny` -> `echo_any`
- `optParamAllowsNull` -> `opt_param_allows_null`

### 3) C ABI 导出符号
- 采用 **snake_case**，并保持稳定前缀：
  - singleton ctor：`ridl_create_<singleton_name_snake>_singleton`
  - class ctor（如有）：`ridl_create_<class_name_snake>`（待确定）
  - proto hook（如有）：`ridl_create_proto_<class_name_snake>` / `ridl_drop_proto_<class_name_snake>` / `ridl_proto_get_<class_name_snake>_<field>`

示例：
- `TestDiagnostics` -> `ridl_create_test_diagnostics_singleton`
- `TestFn` -> `ridl_create_test_fn_singleton`

> 注意：本计划不做旧符号兼容（不生成旧名字 wrapper）。

## 影响范围

### 必须修改

1. `deps/ridl-tool` 代码生成器：
- 生成 `api.rs` 中 trait/class/方法名
- 生成 `glue.rs` 中对 trait 方法的调用、导出符号名

2. `deps/mquickjs-rs`（如存在依赖命名字符串/宏路径假设）：
- `ridl_include_module!` 相关 glue 引用路径是否依赖特定命名

3. 测试模块与功能模块手写 impl：
- `tests/global/**/src/*_impl.rs`
- 以及任何使用生成 trait 名/方法名/ctor 名的模块

4. app 侧聚合/注册：
- `ridl-builder` 生成的聚合代码或注册头文件是否依赖 ctor 名约定

### 可能受影响
- 文档中的示例代码
- 未来 module_mode 测试

## 实施步骤（建议顺序）

1) 在 `deps/ridl-tool` 实现并统一使用命名转换函数：
- `to_upper_camel_case(ident: &str)`
- `to_snake_case(ident: &str)`
- 明确缩写与数字规则

2) 修改 generator：
- `api.rs`：trait/class/方法/类型名全部切换到新规则
- `glue.rs`：
  - 调用 trait 方法时使用 snake_case
  - ctor/导出符号使用 snake_case

3) 批量修复模块实现（以生成产物为准）：
- global_mode 测试模块优先（确保测试先恢复）
- 其它模块（stdlib 等）同步对齐

4) 回归验证：
- `cargo run -p ridl-builder -- prepare`
- `cargo run -- tests`
- `cargo test`

## 测试策略

- 增加 ridl-tool 层的单测：给定若干 RIDL 标识符，断言转换结果稳定：
  - `TestFn` -> `TestFn` / `test_fn`
  - `getName` -> `get_name`
  - `JSValue` -> `js_value`
  - `URLValue2` -> `url_value2`
- JS 集成测试：现有 global_mode 用例能跑通即可；后续在 P0 其它项修复后逐步恢复断言。

## 风险与回滚

- 该变更是破坏性的：任何依赖旧符号的 crate 都会编译失败。
- 本仓库不引入兼容层，因此需要在一个提交序列内一次性完成。
- 如出现不可预期的大面积破坏，将暂停并与用户确认是否拆分为更小步骤。
