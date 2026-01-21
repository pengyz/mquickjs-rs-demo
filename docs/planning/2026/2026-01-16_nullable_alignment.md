<!-- planning-meta
status: 未复核
tags: ridl, types
replaced_by:
- docs/ridl/overview.md
-->

> 状态：**未复核**（`ridl` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
>
> 关键结论：
> - （待补充：3~5 条）
# Nullable 语义对齐方案（2026-01-16）

## 背景

当前 RIDL 的类型系统里已有 `Type::Optional(Box<Type>)`（语法为 `T?`），同时也支持 `Type::Union(Vec<Type>)`（语法为 `A | B | ...`）。

我们希望将 nullable 的实现与讨论方案完全对齐，减少隐式语义，统一生成侧/校验侧对 `null` 的处理。

## 目标语义（最终约束）

### 1) `any` 的可空性

- `any` **默认允许 `null`**。
- 不要求写 `any?`。

> 设计动机：`any` 用于放宽类型约束；strict mode 对 `any` 的使用范围另有约束，但不在 nullable 语义里额外限制 `null`。

### 2) `null` literal 的赋值规则（禁止隐式 nullable）

- `null` 只允许赋值给：
  - `T?`（Optional）
  - `null`（Type::Null）
  - `any`（Type::Any）
- **不支持**把 `null` 赋值给非 nullable 的 `T`，也不做“引用类型默认可空”等隐式规则。

### 3) union 的可空性：仅允许“整体可空/整体不可空”

- 对于 union 类型 `U = A | B | ...`：
  - 要么整体可空，要么整体不可空。
  - **不支持**成员级别的可空（例如 `string? | int`），认为其语义没有意义并禁止。

- 允许显式写 `A | B | null` 来表达“union 整体可空”。
- 规范化：
  - `A | B | null` **等价于** `(A | B)?`。
  - 实现上应规范化为：`Type::Optional(Box::new(Type::Union(vec![A, B])))`。

### 4) 生成侧行为

- 如果接口/函数参数类型为 `T?`（Optional）：生成 glue/转换代码应放行 `null`。
- 如果参数类型为非 Optional：生成 glue/转换代码应拒绝 `null`（报错/抛异常）。

## 实现对齐点（需要修改的代码位置）

### A. parser / AST 规范化

1. 当 parser 解析 union 时：
   - 若 union 成员包含 `null`：
     - 先从 union 成员中移除 `null`；
     - 将剩余成员构造成 `Union(...)`；
     - 最终返回 `Optional(Union(...))`。
   - 若 union 仅包含 `null`：
     - 语义上等价于 `null`（也可以表示为 `Optional(Null)`，但建议归一为 `Null`）。

2. 禁止 union 成员出现 Optional：
   - 若解析到 `Union([..., Optional(T), ...])`：应在 validator 阶段报错（parser 可不做强行拒绝，避免 grammar 复杂化）。

### B. validator 规则

1. `null` literal 赋值：
   - 仅允许目标类型为 `Optional(_)` / `Null` / `Any`。

2. union 的 Optional 成员禁止：
   - 若 `Union(types)` 中存在 `Optional(_)`：报错（给出明确错误信息）。

3. union 的 `null` 规范化一致性：
   - 允许用户写 `A | B | null`。
   - 但在 validator 里应保证后续处理看到的是 `Optional(Union(...))` 的形态。
   - 若 parser 未做规范化，则 validator 做一次规范化；若 parser 已做，则 validator 只做断言与一致性检查。

### C. generator / glue 转换

- Optional 类型：
  - 输入参数检查：允许 `null`。
  - 非 null 时递归按 inner 类型转换/校验。

- 非 Optional 类型：
  - 输入参数检查：拒绝 `null`（抛类型错误）。

> 注意：此处不展开所有语言侧细节，遵循现有 generator 的类型映射/检查框架，在其中补齐 Optional/Union 的一致处理。

## 测试矩阵（必须覆盖）

### parser/validator

1. `any`：
   - `fn f(x: any)` 传 `null` 允许。

2. `null` literal 赋值：
   - `var x: string = null` 失败
   - `var x: string? = null` 通过
   - `var x: any = null` 通过

3. union 整体可空：
   - `fn f(x: string | int | null)` 允许传 `null`，且等价于 `(string | int)?`。

4. union 成员 optional 禁止：
   - `fn f(x: string? | int)` 报错（明确提示“union 仅允许整体可空，不允许成员级 optional”）。

### 生成/运行（JS 集成）

- 增加（或调整）JS 用例：
  - `string | int | null` 参数传 `null` 不报错。
  - `string | int` 参数传 `null` 报错。

## 风险与迁移

- 如果历史上已有 ridl 文件使用 `string? | int` 这类写法，需要统一迁移到 `(string | int)?` 或 `string | int | null`。
- 对于 `Union([... , Null])` 的表现形式应被规范化，避免生成侧出现多种等价形态。

## 完成标准

- ridl-tool：`cargo test -p ridl-tool` 全绿。
- workspace：`cargo test` 全绿。
- JS 集成：`cargo run -- tests` 全绿。
- 文档：本规划文档落地并在实现完成后标记完成。
