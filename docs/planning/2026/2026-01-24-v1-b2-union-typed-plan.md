# V1-B2：union typed（完整实现）计划

> 日期：2026-01-24

## 0. 目标

在 V1 范畴内把 `union` 作为 **typed Rust boundary** 完整打通：
- 允许 union 出现在：函数参数 / 返回值 / Optional(union)
- 覆盖 global 与 module(require) 两种形态
- 提供端到端用例覆盖与回归命令

## 1. 范畴与语义边界（本次约定）

### 1.1 union 成员类型

支持（全覆盖）：
- primitive：`bool` / `int` / `double` / `string`
- `any`
- `object`
- `ClassRef`（类引用）
- `array<T>`（递归支持：T 可以是上述类型或嵌套 union/Optional）
- `Optional(T)`（包括 `Optional(union)`）
- 嵌套 union（例如 `(int | (string | bool))`）

不支持（本次明确排除）：
- `map<K,V>`（因为 map 还未实现）

### 1.2 number 判别规则（JS → union）

JS 侧 `number` 进入 union 时：
- 若值是“整数且可表示为 i32”（范围内且无小数部分），归入 `int` 分支
- 否则归入 `double` 分支

### 1.3 object 判别规则

`object` 视为“非 primitive 的 any”：
- 排除：`null` / `undefined` / `bool` / `number` / `string`
- 其余（包括 object/array/function 等）都算 object

> 备注：`null` 仅通过 Optional(union) 承载（或 union 显式包含 null 时被 normalize 成 Optional）。

## 2. 实现策略

### 2.1 代码生成产物（Rust 侧）

- 对每个出现的 union 成员集合，生成一个稳定命名的 Rust enum：
  - 例如：`UnionBoolIntDoubleStringAnyObject...`
  - 成员包含：
    - `Bool(bool)` / `Int(i32)` / `Double(f64)` / `String(String)`
    - `Any(Local<'_, Value>)`（param 侧）或 `ReturnAny`（return 侧，视现有 ABI 约束）
    - `Object(Local<'_, Value>)`（或同 Any，但 decode 需额外判别）
    - `ClassX(Box<dyn ...>)`（由现有 ClassRef 规则决定）
    - `Array(Vec<...>)`（递归）

- Optional(union) 在 Rust 边界表示为 `Option<UnionEnum>`

### 2.2 glue（JS ↔ Rust）

- 参数 decode：
  - Optional(union)：`null/undefined => None`；否则 decode 成 `Some(UnionEnum)`
  - union：按成员类型顺序进行判别/转换
    - number：按 §1.2 分流 int/double
    - string/bool：直映射
    - classRef：按现有 class decode 路径优先判别
    - object：按 §1.3 判别
    - any：兜底（仅当 union 明确包含 any 时）

- 返回 encode：
  - `UnionEnum`：match 分支逐个转 JSValue
  - `Option<UnionEnum>`：None => JS null；Some => encode

## 3. 用例覆盖

### 3.1 global/types

新增/扩展用例覆盖：
- union param：`handleUnion(x: int | double | string | bool | object | any)`
- union return：`makeUnion(tag: int) -> (int | double | string | bool | object | any)`
- Optional(union) param/return：
  - `maybeUnion(x: (int|double|string)? ) -> (int|double|string)?`
  - 覆盖：undefined/null/具体值
- number 判别：
  - `1 => int`；`1.1 => double`；`(1<<31) => double`
- object 判别：
  - `{}`、`[]`、`()=>{}` 进入 object 分支
  - `null` 只能在 Optional 路径
- array 成员：`array<int|string>` param/return（递归）
- 嵌套 union：`int | (string | bool)` 的 decode/encode

### 3.2 module/basic

至少新增 1-2 个关键用例覆盖同样能力：
- module 导出的 class/函数中包含 union typed param/return

## 4. 验收

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

全部通过，并且新增的 global/module 用例覆盖上述判别规则。
