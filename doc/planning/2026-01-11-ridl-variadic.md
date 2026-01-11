# RIDL 可变参（variadic）语法与 Glue/ABI 约定（设计）

> 范围：为 `console.*` 与 timers 等 Node-ish 接口提供可用的多参数调用形式。
>
> 约束：mquickjs 运行时以 ES5 为主，但宿主接口属于“实现扩展”，不受 ECMA 规范返回类型限制。

## 背景与动机

在 Node 生态中，`console.log(...args)`、`setTimeout(cb, delay, ...args)` 等 API 是事实标准。

若 RIDL 仅支持固定参数（如 `log(message: string)`），则会导致：
- 与开发者预期差距大
- 需要在 JS 侧手写 wrapper（不利于“stdlib 补全”的目标）

因此需要在 RIDL 语法层面引入可变参，并明确 glue 的参数传递方式。

## 目标

- RIDL 支持函数末尾可变参。
- 生成的 glue 代码能在 C ABI 入口收到 `argc/argv` 并正确处理。
- v0 先满足 console 与 timers 的最低可用；复杂对象格式化（inspect）后续再迭代。

## 语法提案

### 方案 A（推荐）：`...` 只能出现在参数列表末尾

```ridl
class Console {
  fn log(...args: any);
  fn error(...args: any);
}

singleton console: Console;

fn setTimeout(cb: callback, delay: int, ...args: any) -> int;
fn setInterval(cb: callback, delay: int, ...args: any) -> int;
```

规则：
- `...args: T` 必须是最后一个参数。
- `T` 初期可限制为 `any` 或 `string`（见下一节）。

### 方案 B（更保守）：仅支持 `...args: string`

```ridl
fn log(...args: string);
```

含义：
- glue 侧对每个 argv 做 `ToString` 后再传给 Rust。
- 牺牲类型信息，但实现简单。

> 结论：优先采用方案 A 的语法，但实现可以先落在“按 string 处理”的 v0 路径上。

## 类型语义：`any` 的落地方式（分阶段）

由于目前 RIDL 类型系统里没有显式 `any`（文档中主要是 `object`），需要明确：

- v0：把 `any` 在实现层降级为 `object`（即 JSValue），并由 glue 提供最基础的 ToString。
- v1：引入显式 `any`，在 Rust 侧用一个轻量句柄类型（例如 `JsValueRef`）表示，允许后续实现 inspect。

本次 stdlib 补全优先级：console/timers 可用 > 完整类型保真。

## Glue/ABI 约定

### 入口形态

mquickjs 对 CFunction 的惯例是：
- `JSValue func(JSContext* ctx, JSValueConst this_val, int argc, JSValueConst* argv)`

RIDL 生成的 glue 应遵循该入口，并在内部：
- 解析固定参数（cb/delay）
- 剩余参数打包为 slice/vec 传递给 impl

### Rust impl 形态（建议）

以 console 为例：

- v0（string 化）：
  - `fn console_log(args: Vec<String>)`

- v1（保留 JSValue）：
  - `fn console_log(ctx: &Context, args: Vec<Value>)`

以 timers 为例：

- v0（args 仅 ToString，回调只传 string）
  - `fn set_timeout(cb: CallbackId, delay: i32, args: Vec<String>) -> i32`

- v1（args 原样传入回调）
  - `fn set_timeout(cb: JsFunctionRef, delay: i32, args: Vec<Value>) -> i32`

> 具体取决于现有 callback/Value 管理能力。本设计文档只规定 glue 必须能拿到“剩余参数”。

## 与 console.enabled 这类属性的关系

属性不需要 variadic。但若 console.log 的行为受 enabled 控制，则：
- enabled 应作为 singleton 的内部状态（Rust 侧）或 JS 对象字段（C 侧）。
- v0 推荐先做 Rust 侧静态开关（默认 true），后续再做可写属性。

## Open questions

1. 现有 ridl-tool parser 是否已支持 `...` token？若否，扩展语法与 AST 的最小改动是什么？
2. `any` 采用新关键字还是复用 `object`？（避免与 JS Object 混淆）
3. console 的输出格式：v0 是否统一 `ToString` + 以空格拼接？
