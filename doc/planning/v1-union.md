# V1：Union 类型（A | B）设计与实施计划

> 目标：按 V1 合规补齐 Union（不含数值 union）在 ridl-tool 生成与 v1 glue 的端到端支持，并补齐 tests/global 的回归用例。

## 0. 背景与现状

- parser/ast 已支持 `Type::Union(Vec<Type>)`
- validator 已做部分语义限制：
  - **union 成员不允许 optional**（`string? | int` 禁止）
  - 建议写 `(A | B)?` 或 `A | B | null`
- 生成阶段缺口：`deps/ridl-tool/src/generator/filters.rs::rust_type_from_idl()` 对 `Type::Union` 报错：
  - `unsupported ridl type in rust_type_from_idl: Union(...)`
- tests/global/types/test_types 中 union 用例目前被注释。

本计划要求：union 的实现 **单独提交**，不与前一个 V1 types baseline commit 混在一起。

## 1. V1 语义（已确认）

### 1.1 Union 的可空规则

- **union 只能整体可空**，不允许成员级可空。
- 语义等价：
  - `T1 | T2 | null` **等价于** `(T1 | T2)?`

因此 Rust 边界上自然映射为：
- `(T1 | T2)?` -> `Option<UnionEnum>`
- `T1 | T2 | null` -> 同样当作 `Option<UnionEnum>`

### 1.2 禁止数值类型 union

由于 JS 引擎层面 number 只有一种承载（double），`int` 在 RIDL 中只是“约束/说明”，
`int | double` 这类 union 会导致运行时不可判别/规则不一致。

结论：**禁止数值类型 union**。

- 禁止示例（应报 semantic error）：
  - `int | double`
  - `int | float`
  - `float | double`
  - 任意包含多个数值类型分支的 union

对用户的推荐：
- 若不确定数值类型具体形态，直接声明 `double`。

> 注：是否允许 `int | string` 这类“可判别 union”？允许。

### 1.3 default/strict 下的转换原则

- RIDL 是严格语言：default 下也不做“形式类型转换”。
- strict 是额外更严限制（当前主要是禁用 any；未来可扩展）。

对 union：
- **分支选择只依赖可判别的 JS 运行时类型**：
  - `string` 分支：仅接受 JS string
  - `int` 分支：仅接受 JS number 且必须是整数（已在 int 参数规则中定义）
  - `null`：被规范化为 Optional(None)
- 不做隐式转换：
  - `string|int` 传 `true` -> TypeError
  - `string|int` 传 `1.5` -> TypeError（因为 int 分支要求整数）

## 2. Rust 边界类型设计

### 2.1 形态：无 tag enum（untagged enum）

为每个 union 生成一个 Rust enum，例如：

- RIDL: `string | int`
- Rust:

```rust
pub enum TestTypesEchoStringOrInt {
    String(String),
    Int(i32),
}
```

### 2.2 null 归一化为 Option

- RIDL: `string | int | null`
- Rust: `Option<TestTypesEchoStringOrInt>`

不生成 `Null` enum 成员（避免双重可空形态，并与 Optional(T) 一致）。

### 2.3 命名与符号设计（通用准则引用）

本特性（union）遵循仓库通用准则：
- Rust 侧类型/符号用命名空间隔离
- C 导出符号用 `_` flatten
- 引入 global/module 的 type domain 并按规则 normalize

详见：`doc/planning/design_guidelines_naming.md`

## 3. 生成与 glue 策略

### 3.1 validator：新增 union 限制

在 `deps/ridl-tool/src/validator/mod.rs` 的 `Type::Union(types)` 分支增加：

- union 成员级 optional 已有（保留）
- 新增：数值 union 禁止
  - 若 union 成员中数值类型（int/float/double）出现 2 个及以上 -> semantic error
  - 错误信息：
    - `Union 不支持数值类型联合（例如 int | double）。若不确定数值类型，请使用 double。`

### 3.2 generator：rust_type_from_idl 支持 Union

`rust_type_from_idl(Type::Union)` 将返回生成的 enum 名称（而不是直接报错）。

需要配合 Askama 模板：
- 在生成 API trait/impl glue 的同时，生成 union enum 定义。
- enum 定义位置：建议与当前 module 的 `out/api.rs` 同层，或生成到单独文件再 `mod` 引入。

### 3.3 v1 glue：参数解码

对 union 参数：
- 先处理 optional 化：
  - 若 union 含 null（或语法为 Optional(union)）-> 统一走 `Option<UnionEnum>`
  - `null/undefined` -> None
- 否则按分支顺序尝试解码（顺序固定且可预测）：
  - `string` 分支优先于 `int`（因为 `int` 只接受 number，`string` 只接受 string，不冲突）
  - 对每个分支：使用现有单类型解码逻辑（string/int 等）
- 全部分支失败 -> TypeError

### 3.4 v1 glue：返回编码

对 union 返回：
- 若 Rust 返回的是 `Option<UnionEnum>`：
  - None -> JS_NULL
  - Some(x) -> 递归把 enum 分支编码成 JS 值
- 若 Rust 返回的是 `UnionEnum`：
  - 按分支编码成 JS 值

## 4. 测试矩阵与完整 RIDL 示例（先写用例再实现）

### 4.1 tests/global/types/test_types（default）

在 `tests/global/types/test_types/src/test_types.ridl` 启用：

```ridl
singleton TestTypes {
    fn echoStringOrInt(v: string | int) -> string | int;
    fn echoStringOrIntNullable(v: string | int | null) -> (string | int)?;
}
```

JS 用例（tests/global/types/test_types/tests/basic.js）：
- `echoStringOrInt('hi') === 'hi'`
- `echoStringOrInt(123) === 123`
- `echoStringOrInt(1.5)` 抛 TypeError
- `echoStringOrIntNullable(null) === null`
- `echoStringOrIntNullable(undefined) === null`

Rust impl：
- 参数/返回直接 roundtrip（match enum 分支并返回同分支）。

### 4.2 tests/global/diagnostics（strict 语义）

本轮 union 的 strict 差异主要体现在：
- 若未来 strict 禁止 any/更多类型转换，这里补充相应用例。

当前明确的 strict 用例：
- `int` 分支拒绝 `1.5`（已在 default 下也拒绝，因此 diagnostics 可只做“报错路径/错误信息”覆盖，是否必须另起 strict 用例待确认）。

> 由于我们已将 int 在 default 下也定义为“只接受整数”，严格性已提升，diagnostics 中可转而覆盖 union 的“不匹配类型”报错定位。

## 5. 交付与验证

- 实现顺序：validator -> generator enum -> glue param/return -> tests
- 验证命令：
  - `cargo run -p ridl-builder -- prepare`
  - `cargo test`
  - `cargo run -- tests -q`

## 6. 提交策略

- union 作为独立工作量：
  - 单独 commit（不 amend 到现有 commit）
  - commit message 遵循规范：subject < 50 列，body < 88 列，偏 why
