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
# RIDL：类型子语法从 Pest 迁移到 Pratt/优先级解析器（方案草案）

> 日期：2026-01-17

## 1. 背景与动机

当前 RIDL 的整体语法（module/interface/class/成员/参数列表等）使用 Pest（PEG）解析。

在类型表达式（`type`）部分，我们需要支持：

- 中缀运算：`|`（union）
- 后缀运算：`?`（nullable / Optional）
- 分组：`(...)`
- 复合类型：`array<T>`、`map<K,V>`、`callback(...)` 等

在实践中已经出现结构化信息丢失：例如 `(string | int)?` 可能退化为 `Optional(Custom("(string | int)"))`，导致 validator/generator 无法识别 union，从而泄漏不合法 Rust 类型（如 `Option<(string | int)>`）或触发 unsupported glue。

根因是“表达式优先级/结合性”的类型语法与 Pest 的“无左递归”限制不匹配：一旦尝试让 `union_type` 在 `primary_type` 中可递归出现（以便 group 内可结构化解析 union），会形成间接左递归环而被 Pest 拒绝。

## 2. 目标

- 仅迁移 **类型子语法**（`type`）到专用的表达式解析器（Pratt / precedence climbing）。
- 保持文件级结构语法仍由 Pest 提供（低风险、低回归）。
- 产出稳定的 Type AST，不再出现 “把可解析表达式吞成 Custom 字符串” 的退化形态。
- 将语法糖归一化（canonicalization）纳入 parser 的标准流程，使 validator/generator 可以依赖不变量。

## 3. 范围（Scope）

### 3.1 保持 Pest 的部分

- module/mode 声明
- interface/class/enum/struct/singleton 等定义
- method/function 结构
- param_list / 标识符 / 字符串字面量

这些都是“块状语法”，PEG 表达简洁且稳定。

### 3.2 迁移到 Pratt 的部分

- `type` 表达式的解析：
  - `|` union
  - `?` nullable
  - `(...)` group
  - `array<type>` / `map<type,type>`
  - `callback (...)`（若 callback 的参数类型也复用 type，则一并覆盖）

实现方式：
- Pest 仍然在 grammar 层保留 `type` 的匹配入口，但不再试图在 grammar 中完整表达优先级；而是将 `pair.as_str()`（或 token 序列）交给类型解析器。

## 4. 语义与不变量（Invariants）

### 4.1 语义策略（与当前 union 语义保持一致）

- 策略A：`T1 | T2 | null` 等价于 `(T1 | T2)?`，Rust 映射为 `Option<UnionEnum>`。
- union 只能“整体可空”：nullable 通过 `Optional(Union(...))` 表达。

### 4.2 AST 不变量（解析完成后必须满足）

1) union 必须以 `Type::Union(Vec<Type>)` 表达，禁止以 `Type::Custom("...")` 隐藏 union。
2) nullable union 统一为 `Type::Optional(Box::new(Type::Union(...)))`。
3) `Type::Group` 仅用于保留分组语义（若下游不需要分组信息，可在 canonicalization 中剥离）。
4) `Type::Optional(Optional(_))` 不出现（避免双 Optional），在 canonicalization 中折叠。

## 5. Pratt 解析器设计

### 5.1 Token 设计（建议）

把 `type` 字符串切分为 token：

- Ident（例如 string/int/float/double/object/any/null，以及用户自定义类型名）
- Symbols：`|` `?` `(` `)` `<` `>` `,`
- Keywords：`array` `map` `callback`

### 5.2 优先级/结合性

- 后缀 `?`：最高优先级，左结合（`T??` 不允许或归一化为一次 Optional）
- 中缀 `|`：低优先级，左结合（`A|B|C` 解析为 Union(A,B,C)）
- 分组 `(...)`：改变优先级

### 5.3 复合类型

- `array<T>`：识别 keyword `array` + `<` + type + `>`
- `map<K,V>`：识别 keyword `map` + `<` + type + `,` + type + `>`
- `callback (...)`：按现有语义组装 Type

## 6. Canonicalization（规范化）阶段

即使采用 Pratt，也建议保留一个轻量 canonicalization pass，以便：

- `Union(..., Null)` -> `Optional(Union(...))`
- 去重/排序（如果 union enum 命名依赖成员集合）
- 去除 `Group`（若无必要）
- 合并嵌套 Optional

该 pass 是“通用流程”，不是 workaround；其定位是把多种等价语法统一到同一 AST 表达。

## 7. 测试矩阵

### 7.1 Parser 层单测

- `string|int|null` -> `Optional(Union(string,int))`
- `(string|int)?` -> `Optional(Union(string,int))`（或 Optional(Group(Union))，再 canonicalize）
- `array<string|int>`
- `map<string, (int|string)?>`

### 7.2 Generator 层快照/断言

- 生成 api.rs/glue.rs 不含 `(string | int)` 这样的原始 RIDL union 文本
- nullable union 统一映射 `Option<...UnionEnum>`

### 7.3 主仓库端到端

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

## 8. 迁移步骤（建议迭代）

1) 先引入 canonicalization（已作为当下修复 union 的必要步骤），确保下游稳定。
2) 实现类型 tokenization + Pratt parser（仅限 `type`）。
3) 将 `parse_type` 的实现切换为 Pratt 产出的 AST。
4) 保持旧 grammar 入口不变，减少调用面改动。
5) 全量测试 + 回归，移除旧的“字符串修复类”代码路径。

## 9. 风险与回退

- 风险：类型解析是高频路径，错误会放大到所有生成物。
- 回退策略：保留旧解析器实现一段时间（feature flag 或内部切换），便于对比与快速回滚。

---

> 备注：本方案不要求“全量脱离 Pest/字符流解析”；推荐的长期架构是：结构语法由 Pest，表达式子语法由 Pratt。
