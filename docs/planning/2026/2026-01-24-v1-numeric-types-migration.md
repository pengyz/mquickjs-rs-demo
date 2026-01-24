# V1：数值类型迁移（RIDL → Rust 风格）计划

> 日期：2026-01-24

## 0. 背景与目标

当前 RIDL 的基础数值类型为：`int` / `float` / `double`。
我们决定改为 Rust 风格：`i32` / `i64` / `f32` / `f64`，并且 **不做语法兼容**（旧类型名直接移除）。

目标：
- RIDL 语法层仅支持：`i32` / `i64` / `f32` / `f64`
- 端到端：parser/AST/validator/codegen/runtime glue/tests 全部切换并通过回归

## 1. 语义规则（本次约定）

### 1.1 JS number → 整数/浮点

JS 只有 `number`（IEEE754 double），因此 decode 规则如下：

- `i32`：
  - 必须是 finite number
  - 必须是整数（`fract()==0`）
  - 必须在 i32 范围内
  - 否则抛 TypeError

- `i64`：
  - 必须是 finite number
  - 必须是整数
  - 必须满足 `abs(n) <= 2^53 - 1`（JS number 的安全整数范围）
  - 再检查 i64 范围（理论上一定满足，但保留实现检查）
  - 否则抛 TypeError

- `f32`：
  - 必须是 finite number
  - `JS_ToNumber` 得到 f64 后 cast 为 f32
  - 若出现 `NaN/Inf`：按 V1 strict 规则拒绝（TypeError）

- `f64`：
  - 必须是 finite number
  - `JS_ToNumber` 得到 f64

> 备注：这里的“finite”沿用现有 strict 语义（不接受 NaN/Inf）。

### 1.2 Rust → JS number

- `i32/i64/f32/f64` 都 encode 为 JS number
  - `i32`：`JS_NewInt32`
  - `i64`：优先 `JS_NewInt64`（若 QuickJS API 可用；否则用 `JS_NewFloat64` 并在文档中注明精度边界）
  - `f32/f64`：`JS_NewFloat64`

## 2. 影响面（必须同步修改）

- RIDL grammar：basic_type/token 切换为 i32/i64/f32/f64
- AST Type 枚举：替换 Int/Float/Double 为 I32/I64/F32/F64
- normalize/validator：所有类型匹配与规则分支同步调整
- generator/filters：
  - 参数 decode：ToI32/ToI64/ToF32/ToF64
  - 返回 encode：NewInt32/NewInt64/NewFloat64
  - Optional(any)/any? 语义不变
- templates（rust_api.rs.j2 / rust_glue.rs.j2 / headers）：类型名与 glue 调用更新
- tests：所有 *.ridl / Rust impl / JS 断言更新

## 3. 实施步骤

1) 修改 grammar + parser + AST
2) 修改 normalize/validator
3) 修改 generator（filters + 相关模板）
4) 更新 tests（global/module）
5) 跑回归：
   - `cargo run -p ridl-builder -- prepare`
   - `cargo test`
   - `cargo run -- tests`

## 4. 验收标准

- 仓库内不再出现 `int/float/double` 的 RIDL 基础类型用法（除非在注释/历史文档中明确说明）
- 回归命令全部通过
- JS number→i64 的边界行为可回归（至少覆盖：
  - `2^53-1` OK
  - `2^53` reject
  - 小数 reject
  - NaN/Inf reject）
