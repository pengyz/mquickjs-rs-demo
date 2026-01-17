# Phase A：test_types（V1 合规）测试矩阵与 RIDL 示例

本文档用于 Phase A 工作流的“先审阅后实现”。

- 允许省略 `mode` 表示 default。
- `mode strict;` 必须显式写。
- 本阶段只做 **V1** 范围；module 模式归 V2。

> 备注：当前仓库的 `tests/global/types/test_types` 仍很薄（只测 any 透传 null）。Phase A 的目标是把它扩成 V1 类型系统回归基线。


## 1. 本阶段验收命令

- `cargo run -p ridl-builder -- prepare`
- `cargo run -- tests`
- `cargo test`


## 2. 测试矩阵（按“基础 → 组合”逐步落地）

> 已确认语义：
> - 省略 `mode` 即 default。
> - `mode strict;` 必须显式写。
> - union 的 number 行为：**strict 下禁止形式类型转换**（`int` 只能接收整数）；default 下允许小数（按 number 处理）。
> - any 透传为 Rust `handles::Local<handles::Value>`，遵循最小惊讶原则：**primitive 保持类型/值，object 保持引用 identity**。

### 2.1 基础类型（default）

覆盖目标：
- primitive 参数/返回的 roundtrip：`bool/int/float/string`

建议用例：
- `echoBool(v: bool) -> bool`
- `echoInt(v: int) -> int`
- `echoFloat(v: float) -> float`
- `echoString(v: string) -> string`

JS 断言：
- `bool/int/string`：`===` 严格相等
- `float`：JS 侧用 `Number.isFinite` + 近似比较（或选用可精确表示的值如 `1.5`）


### 2.2 nullable（default）

覆盖目标：
- `T?` 参数可传 null
- `T?` 返回可返回 null

建议用例：
- `echoStringNullable(v: string?) -> string?`
- `echoIntNullable(v: int?) -> int?`

JS 断言：
- `null` roundtrip
- 非 null roundtrip


### 2.3 any（default）

覆盖目标：
- any 参数/返回对 primitive 保持类型与值
- any 对 object 保持引用（identity）

建议用例：
- `echoAny(v: any) -> any`

JS 断言（primitive）：
- `null`：`ret === null`
- `bool`：`ret === true/false`
- `number`：覆盖整数与小数（default 下允许小数），`ret === input`
- `string`：`ret === input`

JS 断言（object identity）：
- `let obj = { a: 1 }; let ret = t.echoAny(obj);`
  - `ret === obj`
  - `ret.a === 1`

> 注：当前 `test_types.ridl` 里 `echoAny(v:any)->void`，会改成返回 any 以便做 roundtrip。


### 2.4 union（default + strict 对比点先记录）

覆盖目标：
- `string | int` 的传参/返回
- 含 null 的 union：`string | null`

建议用例：
- `echoStringOrInt(v: string | int) -> string | int`
- `echoStringOrNull(v: string | null) -> string | null`

JS 断言：
- default：
  - 传 string：返回 string 且值一致
  - 传 int：返回 number 且值一致
  - 传 `1.5` 到 `string|int`：按 default 规则允许（作为 number）
- strict：
  - 该对比用例放到 Phase B/diagnostics（在 strict 下传 `1.5` 给 `int` 分支应失败）


### 2.5 strict 行为（本阶段不做实现；但语义已确定）

Phase A 只做 default 的“可用性”与“转换正确性”。
strict 的拒绝/报错断言，归入 `tests/global/diagnostics`（Phase B）。


## 3. 完整 RIDL 示例（拟）

文件位置建议：`tests/global/types/test_types/src/test_types.ridl`

```ridl
// default mode (mode omitted)

singleton TestTypes {
    // 2.1 primitives
    fn echoBool(v: bool) -> bool;
    fn echoInt(v: int) -> int;
    fn echoFloat(v: float) -> float;
    fn echoString(v: string) -> string;

    // 2.2 nullable
    fn echoStringNullable(v: string?) -> string?;
    fn echoIntNullable(v: int?) -> int?;

    // 2.3 any
    fn echoAny(v: any) -> any;

    // 2.4 union
    fn echoStringOrInt(v: string | int) -> string | int;
    fn echoStringOrNull(v: string | null) -> string | null;
}
```


## 4. JS 用例规划（拟）

文件位置建议：`tests/global/types/test_types/tests/basic.js`

- primitives：分别断言返回值
- nullable：覆盖 null 与非 null
- any：覆盖 null/bool/number/string
- union：覆盖 string/int/null 分支


## 5. 需要你确认的点（避免实现时走偏）

1) `float` 在 V1 中的 JS 表现是否就是 number（IEEE 754 double）？断言采用近似比较可以吗？
2) union 的语义：`string | int` 在 JS 侧接收 number 时是否允许小数？还是只允许整数？
3) `any` 的 roundtrip 是否要求“保持原始 JS 类型”（例如 number/string/bool/null），还是允许某些归一化？

你确认上述 RIDL 示例与矩阵没问题后，我再开始实施（改 RIDL、补 Rust impl、补 JS tests、跑测试、修 bug）。
