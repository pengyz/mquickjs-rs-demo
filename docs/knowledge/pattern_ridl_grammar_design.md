---
name: ridl-grammar-design-principles
description: RIDL PEG 语法设计原则——关键字优先、左递归规避、类型优先级
type: pattern
created: 2026-09-04
sources: [deps/ridl-tool/src/parser/grammar.pest]
---

## RIDL 语法设计原则（PEG/pest）

### 1. 关键字优先于标识符

```pest
keyword = _{ "interface" | "class" | "enum" | ... | "opaque" | "Traced" }
identifier = @{ !keyword ~ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }
```

**规则**：`identifier` 必须用 `!keyword` 负前瞻排除关键字。否则 `class` 会被解析为标识符。

### 2. 类型优先级（避免歧义）

```pest
type = { union_type | nullable_type | primary_type }
primary_type = { traced_type | array_type | map_type | callback_type | group_type | basic_type | custom_type }
```

**优先级从高到低**：
1. `union_type`（`A | B`）—— 最宽泛，先尝试
2. `nullable_type`（`T?`）—— 单类型 + `?`
3. `primary_type` —— 具体类型
   - `traced_type`（`Traced<T>`）—— 在 `basic_type` 前，避免 `Traced` 被解析为 `custom_type`
   - `basic_type`（`bool`, `i32`, ...）—— 在 `custom_type` 前，避免关键字冲突
   - `custom_type`（用户定义类型）—— 兜底

**教训**：`basic_type` 必须在 `custom_type` 之前，否则 `bool` 会被解析为自定义类型。

### 3. 左递归规避

PEG 不支持左递归。`union_type` 用 `primary_type ~ ("|" ~ primary_type)+` 而非递归定义。

### 4. 可空性处理

```pest
nullable_type = { (traced_type | basic_type | ...) ~ (WS? ~ "?") ~ (!"?") }
```

**负前瞻 `(!"?")`** 防止 `T??` 被解析为 `T?` + `?`。

### 5. 语义关键字 vs 语法关键字

- **语法关键字**（在 `keyword` 中）：`class`, `interface`, `fn`, `opaque`, `Traced`
- **语义关键字**（不在 `keyword` 中）：`readonly`, `property`, `proto`, `const`, `var`

`readonly` 等不在 `keyword` 中，允许用户定义名为 `readonly` 的标识符（虽然不推荐）。

### 6. Opaque 块设计

```pest
opaque_block = { "opaque" ~ WS ~ "{" ~ (WS ~ opaque_field ~ WS ~ (";" | ",")?)* ~ WS ~ "}" }
opaque_field = { identifier ~ WS ~ ":" ~ WS ~ type }
```

- 分隔符可选（`;` 或 `,`），宽松解析
- 字段类型复用 `type` 规则，自动支持 `Traced<T>`、`Option<Traced<T>>` 等
