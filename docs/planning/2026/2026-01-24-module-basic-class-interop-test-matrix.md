# module_basic（V1 module 形态）class 互操作测试矩阵（建议稿）

- 状态：部分落地
- 现行替代：无（本文件为 planning，非 SoT）
- 目标：逐步补齐 `tests/module/basic` 覆盖，确保 V1 module 形态下：
  - 生成/构建通过
  - JS 运行断言通过
  - 尽量同时覆盖 ROM + runtime 两条路径（若同一用例能同时命中两条路径更佳）

## 背景

当前重点是 `module_basic`（`test_module_basic`）在 **导出多个 class** 时的互操作能力与稳定性验证。

## 覆盖矩阵（从易到难）

> 说明：每个点都应至少包含：构建通过 + JS 运行断言通过。

### 0. 基础与可观测性

- [ ] **[B0] module 可被 import/require**：
  - `import * as m from "test_module_basic"`
  - `require("test_module_basic")`
- [ ] **[B1] module 导出对象存在**：`typeof m === "object"`
- [ ] **[B2] `__ridl_modules` 中可见**（若这是 V1 约定）：存在对应 key，且版本/元信息正确

### 1. 导出（exports）语义（最小集合）

- [ ] **[E0] 仅导出一个 class（named export）**
- [x] **[E1] 导出多个 class（named exports，至少 2 个）**：`export class A`, `export class B`
- [ ] **[E2] 导出函数 + class 混合**：`export function f()` + `export class A`
- [ ] **[E3] 导出常量/字段**（若 V1 支持）：`export const X`
- [ ] **[E4] 导出别名**（若 V1 允许）：`export { A as A1 }`

### 2. Class 构造与实例行为（单 class）

- [ ] **[C0] `new A()` 成功；`instanceof A` 为 true**
- [ ] **[C1] 原型方法**：`a.m()` 返回预期；参数/返回覆盖基础类型（bool/int/double/string/any）
- [ ] **[C2] getter/setter**（若 V1 支持 class fields）：读写一致；写入类型错误有诊断
- [ ] **[C3] 静态方法/静态字段**（若 V1 支持）：`A.s()`/`A.X`
- [ ] **[C4] 异常/诊断**：参数数量不匹配、类型不匹配、nullability（nullable string/int）行为

### 3. 多 class 相互作用（module 内）

- [x] **[MC0] A 方法返回 B 实例**：`A.makeB()`；JS 侧能 `instanceof B`
- [x] **[MC1] A 接受 B 作为参数**：`A.useB(b)`；验证跨 class 传递不丢类型信息
- [ ] **[MC2] 两个 class 同名方法/字段不会互相污染原型**
- [ ] **[MC3] 多 class 导出顺序不影响可用性**（ROM/注册顺序相关）

### 4. 命名与 id/路径规范（V1 关键稳定性）

- [ ] **[N0] module 名归一化后 class id 正确**：包含 `.` `-` 等；规则：非 `[A-Za-z0-9_]` -> `_`，ALL CAPS
- [ ] **[N1] 多 class 的 class id 唯一且可追踪**（与 `mquickjs_ridl_api.h`/ROM index 联动）
- [ ] **[N2] module 版本字段影响导出命名**（若 V1 有版本段，如 `test.module@1.0`）

### 5. require()/import 边界（module_basic 的“稍难”）

- [x] **[R0] 同一模块重复 require 返回新 module 对象**（无缓存语义）
  - 现状：`tests/module/basic/multi_class.js` 已断言 `require()` 两次返回对象不相等（`m2 !== m3`）。
  - 注意：虽然 module 对象不相等，但其导出成员（例如 class 构造器）应保持引用一致，见 [R1]。
- [x] **[R1] import 与 require 互操作一致**（导出的 class/fn 引用相等）
- [ ] **[R2] require 在 RIDL stdlib normalization 后仍可用**（与 runtime normalization 相关）

### 6. ROM/标准库 materialize 路径（最难但必要）

- [ ] **[ROM0] ROM 模式下 module export 的 class 绑定到 proto + proto_vars 正确**（StdlibInit 路径）
- [ ] **[ROM1] `__ridl_modules` materialize**：module 对象属性可枚举性/可写性符合约定
- [ ] **[ROM2] 多 class 时 ROM class index join 正确**（-M 输出与 generated ids 对齐）

## 当前落地状态（与仓库现状对齐）

- 已通过：
  - [E1] 多 class 导出（`MFoo` + `MBar`）
  - [MC0] `MFoo.make_bar(v) -> MBar`（返回 class）
  - [MC1] `MFoo.use_bar(b: MBar) -> int`（class 作为参数）
  - 相关 JS：`tests/module/basic/multi_class.js`

## 建议落地顺序（从易到难，V1）

1) E0 + C0/C1（单 class 最小闭环）
2) E1 + MC0/MC1（module 导出多个 class + 互相返回/传参）
3) C2/C3/C4（成员/静态/诊断）
4) R0/R1/R2（require/import 边界）
5) ROM0/ROM2（ROM + index 稳定性）

## 下一步（待确认）

- 建议优先补：
  - [B0]/[B1]/[R1]：同一个用例里同时覆盖 `import` 与 `require`，并断言导出的 class/fn 引用一致。
  - [MC2]：增加 `MFoo.get_v()` 与 `MBar.get_v()`（同名方法）或同名 proto var，验证不会互相污染。
