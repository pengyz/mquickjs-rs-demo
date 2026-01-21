<!-- planning-meta
status: 未复核
tags: build, ridl, tests, types
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `ridl` `tests` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# V1：strict mode 下 any 的约束（规划）

> 日期：2026-01-17

## 1. 目标

在 RIDL 的 **strict** 模式下，禁止除“可变参（variadic）”之外的 `any` 使用。

- 允许：`fn f(...v: any) -> void;`
- 禁止：
  - 参数：`fn f(v: any) -> void;`
  - 返回：`fn f() -> any;`
  - 字段/属性：`struct S { x: any; }` / `property x: any;`

该规则属于 **构建期语义约束**，应由 ridl-tool 的 validator 在 build/prepare 阶段直接报错（语义错误）。

## 2. 语义定义

### 2.1 strict 的启用方式

- 允许省略 mode 表示 default。
- strict 必须显式写（例如 `mode strict` 或等价语法，按现有 grammar）。

### 2.2 any 的允许范围

在 strict 模式下：

- 允许：仅当参数是 variadic（可变参）且类型为 any。
  - 语义：`...args: any` 作为“透传容器”，用于接收任意数量 JSValue。

- 禁止：所有非 variadic 的 any：
  - function/method 参数类型为 any
  - function/method 返回类型为 any
  - struct field/property 类型为 any
  - class property 类型为 any
  - singleton property 类型为 any

> 注：如果未来引入 `any?`/`any|null`，也应视作 any 的一种使用并被禁止（除 variadic）。

## 3. 错误策略

- 错误类型：SemanticError
- 错误信息（建议统一）：
  - `strict 模式下禁止使用 any（仅允许可变参 ...args: any）`
- 定位：尽量复用当前 validator 的 file_path/line/column；若当前对类型节点没有精确 pos，则保持 0,0（现状一致），但错误文本必须可读。

## 4. 测试矩阵

### 4.1 ridl-tool validator 单测（推荐最小集）

- strict + non-variadic any param => error
- strict + any return => error
- strict + variadic any param => ok
- default + any param/return => ok（用于确认 strict-only）

### 4.2 tests/global/diagnostics 端到端

新增一个 diagnostics 用例，验证：
- strict 下出现非 variadic any 的 RIDL 模块在 `cargo run -- tests` 的 prepare/build 阶段失败，并输出包含上述错误信息。

（具体“如何在 tests runner 中表达期待失败”的机制需先检查现有 diagnostics 框架；若当前 runner 只支持 PASS 类 JS，用 ridl-tool 单测先兜底，再决定是否扩展 runner 支持 EXPECT_FAIL。）

## 5. 实施步骤

1) 先补 ridl-tool validator 单测。
2) 实现 validator 规则。
3) 再考虑 tests/global/diagnostics 的端到端失败用例支持（如需扩展 runner）。
4) 跑：prepare + cargo test + cargo run -- tests。

## 6. 非目标

- 本规划不引入运行时类型转换或 glue 侧报错（strict 的核心应在构建期收敛）。
- 不在本次修改中迁移 parser 到 Pratt（另有规划文档）。
