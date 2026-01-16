# RIDL：JS-only 字段（var/const）语义与测试规划

日期：2026-01-15

## 背景与目标

现有 RIDL 里的 `property` 会生成 native getter/setter（Rust glue + C-side 注册到 prototype），用于桥接到 Native 实现（opaque/trait）。

但我们需要支持一类**纯 JS 层属性**（不走 native glue，不占用 opaque 状态），用于表达：

- `var`：普通可写 data property
- `const`：只读 data property（不可写）

同时，考虑引擎（QuickJS）对 native class 的限制：native class 实例在语言层面通常仍是可扩展对象，因此 JS-only 字段应当通过**普通属性**实现，而不是 native getter/setter。

## 术语

- **native property**：RIDL `property`（或 `readonly property`）生成的 accessor property，访问会走 Rust/C glue。
- **JS-only 字段**：RIDL `var/const` 生成的 data property，完全在 JS 对象/原型上维护。

## 语法与约束（强制）

### 1) 新增 class_member 语法

在 `class { ... }` 内支持：

- `var <name>: <type> = <literal>;`
- `const <name>: <type> = <literal>;`

### 2) 显式初始化（A）

- `var` 与 `const` **必须显式初始化**（语法层强制包含 `= <literal>`）。
- 仅支持字面量初始化：`string/int/float/double/bool/null`。

### 3) const 与 property 不能组合

- `const` 只能生成 JS data property，禁止与 RIDL `property/readonly property/proto property` 复用同名。
- 若同一 class 内出现同名 `property` 与 `const`（或 `var`）应在 validator 阶段报错。

### 4) 生成位置与可见性

- JS-only 字段默认生成在 **class prototype** 上（所有实例共享）。
  - `var`：writable=true, configurable=true（具体 configurable 可根据现有约定收紧）
  - `const`：writable=false

> 说明：若未来需要 per-instance 初始化，可在 constructor glue 内注入 `this.<name> = <literal>`，本规划先不做。

## 与现有语法的兼容策略

当前 grammar 中 `normal_prop = identifier ":" type` 会在 class_member 中被解析为 ReadWrite Property（native accessor 语义）。

为了避免与 JS-only 字段混淆：

- 建议将 `normal_prop` 从 `class_member` 中移除（或在 strict mode 下禁止），避免用户误写 `var1: int;` 却以为是 JS-only。
- 需要 JS-only 的必须显式写 `var/const ... = ...`。

## 测试矩阵（增量）

### A) 默认构造函数（未声明 constructor）

- class 未声明 ctor，工具补默认 `constructor()`，`new` 成功。

### B) constructor 带参数

- `constructor(a:int, s:string)`：new 后 native property/方法行为正确。
- 错参：缺参、类型不匹配必须抛错。

### C) JS-only var/const

- `var`：读到初值；写入后值变化；不影响 native property。
- `const`：读到初值；写入在 strict mode 下必须抛错（TypeError）；不影响 native property。

### D) proto property（后续）

proto property 依赖模块提供 C ABI（create/drop/get/set）。在约定稳定后，恢复 proto 的强约束测试：
- 未初始化访问行为
- 初始化后读写一致
- 多实例共享语义

## 交付物

- ridl-tool：
  - parser/AST：新增 js-only 成员节点（var/const）或在现有 Property 上区分来源
  - validator：强制初始化与互斥规则
  - C 侧生成：在 class prototype defs 中生成 data property（JS_PROP_*）
- test_class：新增覆盖类与 JS smoke 用例

