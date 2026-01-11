# 计划：用 ridl-tool 重新生成并集成 ridl_module_demo_default（2026-01-08）

## 目标
- 用 ridl-tool 生成的文件替换当前手写/被破坏的示例模块生成代码，使示例模块可正常构建与调用。

## 范围
- 模块：`ridl-modules/ridl_module_demo_default`
- 生成输出：`*_glue.rs`、`*_impl.rs`、`ridl_symbols.rs`、`mquickjs_ridl_register.h` 等与示例模块相关的产物（根目录与 generated 目录同步），必要时更新聚合符号。
- 不改动：上游 `deps/mquickjs`，`DEVELOPING_GUIDE.md`。

## 任务拆解
1) 现状核对：确认示例模块 RIDL 定义和手写/残留生成文件，检查 build.rs 生成链与拷贝路径。
2) 运行 ridl-tool 生成：使用当前 ridl-tool（module + aggregate）重新生成示例模块对应产物。
3) 集成验证：确保生成文件覆盖手写版本，根目录与 generated 同步；检查 `ridl_symbols.rs` 和 `mquickjs_ridl_register.h` 是否包含 sayHello 符号。
4) 构建验证：运行 `cargo build`（或 `cargo check` 如较快）验证通过。
5) 文档同步：若有路径/流程变动，补充到相关 README（已有模板可复用，必要时调整）。

## 风险与关注
- ridl-tool 生成器功能不完备：仅支持函数/接口且参数支持有限（≤3），但示例模块通常定义较小，风险可控。
- 命名规则：聚合使用小写 `js_<name>`，需确认生成命名与聚合一致。
- build.rs 固定 RIDL 列表：此文档为历史计划，现状以当前 build 链路为准。

## 验证标准
- `cargo build` 成功。
- 生成文件中存在 `js_sayhello` (或符合模板命名) 并在 `ridl_symbols.rs` 引用；头文件包含注册声明。
- 根目录与 generated/ 中的相关生成文件一致，无未跟踪垃圾文件。

## 状态
- 进行中（已获确认，开始执行）。
