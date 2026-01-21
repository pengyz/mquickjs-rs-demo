<!-- planning-meta
status: 未复核
tags: context-init, engine, ridl, tests, types
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `ridl` `tests` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 统一 singleton method 与 class method 的强类型 glue（规划）

日期：2026-01-16

## 背景

当前 ridl-tool 对 singleton method 的 glue 生成存在结构性问题：

- singleton trait 方法签名为 `fn method(&mut self, ctx: *mut JSContext, args: Vec<JSValue>)`，glue 侧把 `argv` 原样打包成 `Vec<JSValue>` 透传。
- 与之相对，class method（以及 function glue）的设计目标是：glue 负责把 `argc/argv` 按 RIDL 类型规则转换为强类型 Rust 参数，并把返回值按规则转回 JS。

这导致：
- 参数转换规则在 singleton 路径被绕过（设计不一致）。
- glue 里尝试生成 `emit_param_extract` 会产生未使用变量（warning），进一步暴露“提取-调用链路未闭合”。

用户已确认：singleton method 与 class method 是高度同构的调用流程，**唯一区别**在于 Rust instance 的获取方式（singleton 从 ctx slot 取；class 从 JS opaque 取）。

## 目标

1. singleton method 走与 class method 同一套参数/返回转换规则。
2. singleton 与 class 的差异收敛到一个点：**实例获取函数不同**。
3. 保持现有约束：C API 注册必须编译期完成；不引入运行时 C API 注册。
4. 迁移后：`cargo run -p ridl-builder -- prepare`、`cargo run -- tests`、`cargo test` 全绿。

## 非目标

- 本计划不立即补齐 v1 glue 对所有复杂类型的完整支持（但会给出分阶段落地顺序）。
- 不引入旧命名兼容层。

## 设计概览

### 1) 统一 trait 方法签名

将 singleton trait 方法签名从：

```rust
fn foo(&mut self, ctx: *mut JSContext, args: Vec<JSValue>)
```

改为与 class method 同构的强类型参数：

```rust
fn foo(&mut self, a: T1, b: T2, ...) -> R
```

- 参数类型 `Tn` 与返回类型 `R` 使用既有 `rust_type_from_idl` 规则。
- 重要约束：**singleton trait 不接收 ctx**；ctx 仅存在于 glue 层（用于取实例与转换）。

### 2) 统一 glue 调用骨架

抽象出“从 JS 调用 Rust 方法”的统一流程：

1. 取实例 `&mut dyn Trait`
2. 按 RIDL 参数列表生成 `emit_param_extract`（含 missing arg、类型检查、转换）
3. 调用 `inst.method(p0, p1, ...)`
4. 按返回类型生成 `emit_return_convert`

其中：
- singleton 的 (1) 通过 ctx 的 ridl ext slot 获取 `Box<dyn SingletonTrait>`。
- class 的 (1) 通过 `JS_GetOpaque` 获取 `Box<dyn ClassTrait>`。

### 3) 类型支持策略（分阶段）

建议拆分为 2 个阶段：

**阶段 A（语义对齐，先跑通现有用例）**
- 先让 singleton method 的签名与 glue 调用闭合（强类型化）。
- **类型集合范围说明（阶段A）：仅覆盖当前仓库里 stdlib + tests/global 现有用例实际用到的类型。**
  - 未覆盖到的类型路径继续 `compile_error!`，避免静默错误。
- `any` 在 Rust 边界统一为 `mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>`（不允许用户代码使用裸 `JSValue`）。
- variadic（...）允许，且与 class method 设计一致：Rust 侧签名为 `Vec<T>`。

**阶段 B（补齐类型矩阵）**
- 完整实现 optional / nullable / union 的参数与返回转换，统一覆盖 singleton/class/function。
- 恢复/加强相关 JS 断言。

## 迁移步骤

1. 修改 `deps/ridl-tool/templates/rust_api.rs.j2`
   - singleton trait 方法改为强类型参数列表（含 variadic -> Vec<T>），不再是 `args: Vec<JSValue>`。

2. 修改 `deps/ridl-tool/templates/rust_glue.rs.j2`
   - singleton method glue 使用 `emit_param_extract` 做参数转换，并使用转换后的变量调用 `s.method(...)`。

3. 更新各模块实现（ridl-modules）
   - stdlib、global_mode tests 等：把 singleton impl 方法签名从 `args: Vec<JSValue>` 改为强类型参数列表。

4. 更新/补齐测试
   - 现有 JS 用例保持不变或做最小变更。

5. 验证
   - `cargo run -p ridl-builder -- prepare`
   - `cargo run -- tests`
   - `cargo test`

## 风险

- 这是 API 破坏性变更：所有 singleton 实现需要同步改签名。
- optional/union/nullable 的规则若未在阶段 A 覆盖，会在编译期 `compile_error!` 直接阻断。

## 状态

- [x] 已实现（阶段A）
