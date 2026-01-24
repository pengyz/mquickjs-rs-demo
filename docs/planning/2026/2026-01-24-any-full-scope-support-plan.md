<!--
status: 已完成（通过：cargo test + cargo run -- tests）
owner: Mi Code
tags: ridl, glue, any, tests
-->

# 2026-01-24：any 全范围支持（class/module/singleton）设计与实现计划

> 目标：在 V1 glue 体系下，使 `any` 同时支持
>
> - function（全局函数/模块函数）参数/返回
> - singleton 方法参数/返回
> - class 方法参数/返回
>
> 并通过 tests/ 端到端用例覆盖。

## 0. 背景与现状

### 0.1 当前已经支持的部分

- **全局 singleton / function**：存在 `echoAny`、`anyToString`、`arr*` 等用例，说明 `any` 参数/返回在某些路径上已被实现并可用。
- **class method 参数 any**：近期为解决 `scope/env` 缺失，已在 `rust_glue.rs.j2` 对 `method.needs_scope` 分支补上 `HandleScope + Env`。

### 0.2 当前缺失的部分（核心问题）

- **class method 返回 any**：生成端倾向于采用“两段式 out-callback”接口（例如 trait 同时生成 `foo(...)` 与 `foo_out(...)`），但 class-method 的 glue 模板未完整走通该返回路径，导致无法表达 `-> any`。

> 结论：要实现“全范围支持 any”，必须把 **return any** 的抽象统一起来：
> - 需要时走 `*_out`（out-callback）
> - 不需要时走普通 `-> JSValue`/`-> T` 直返

## 1. 需求定义（Scope）

### 1.1 语义

- `any` 在 JS 侧表示任意 JS 值（包括 object/function/number/string/null/undefined）。
- `any` 在 Rust glue 层表示 QuickJS `JSValue` 的安全封装（`Local<Value>`/`Any<'hs,'ctx>`），并遵守项目约束：
  - 不显式 free/dup `JSValue`（由 tracing GC 管理）。

### 1.2 支持矩阵

| 位置 | param:any | return:any |
|---|---:|---:|
| global fn | ✅ 已有用例 | ✅ 已有用例 |
| singleton method | ✅ 已有用例 | ✅ 已有用例 |
| module fn | ✅ 需要补用例 | ✅ 需要补用例 |
| class method | ✅ 已可编译（needs_scope） | ❌ 需要实现 + 用例 |

## 2. 设计：统一 any return 的 glue 约定

### 2.1 为什么需要 out-callback

`any` 返回值是“上下文相关的 JS 值”。
在当前 handle-scope/escape 语义下，直接在 Rust 层返回一个 `Local<'a, Value>` 往往会受生命周期限制；因此 generator 通过 `*_out` 形式把“返回值的构造与逃逸（escape）”交给 glue 控制。

### 2.2 统一规则

当且仅当返回类型是以下之一时，生成 `*_out` 并让 glue 走 out 分支：

- `any`
- `optional<any>`（若存在）
- `union` 内含 any（若存在）
- `variadic any` 的返回（若未来支持）

其余返回类型保持原有直返路径。

### 2.3 glue 侧 out 分支的目标形态（概念）

- glue 创建 `HandleScope` / `Env`（或使用现有）
- 调用 impl 的 `*_out(env, out, ...args)`
- `out` 回调中接收 `Any<'hs,'ctx>`，并将其转为 `JSValue`（通过 `Any::as_local()/to_local()` 等既有 API；若缺失则补 API）
- glue 将该 `JSValue` 作为 JS 返回值返回给 QuickJS

> 注：具体 API 名称必须以项目中已存在的 `Any/Local/Env` API 为准；实现前需要先在代码库确认。

## 3. 实施计划（分阶段）

### 3.1 Phase A：补齐 generator/glue 对 class method return any 的支持

1) 修改 ridl-tool 的数据模型/filters：
   - 明确 `method.return_type` 为 any 时应生成 `*_out` 且 glue 使用 out 分支。
2) 修改 `templates/rust_glue.rs.j2`：
   - 在 class method glue 中实现 out-callback 的调用与返回值拼装。
3) 在 tests/module/basic 的测试模块增加：
   - `class MFoo { fn echo_any_ret(v: any) -> any; }`
   - Rust impl 实现：`echo_any_ret_out`（确保返回同一个对象，JS 侧用 `===` 验证 identity）。

### 3.2 Phase B：补 module fn 的 any param/return 用例

- 在 `tests/module/basic/test_module_basic.ridl` 增加 `fn echo_any_fn(v: any) -> any;`（module function）
- JS 用例验证 `require('test_module_basic').echo_any_fn(obj) === obj`

### 3.3 Phase C：补 singleton 的 any return 用例（若现有覆盖不足）

- 复用 `tests/global/fn` 或 `tests/global/singleton`，确保存在至少一个“返回对象 identity 保持”的断言（不仅是 stringify）。

## 4. 测试计划

### 4.1 JS 端到端

- 新增/扩展 JS 用例（优先放在现有 `tests/module/basic/multi_class.js` 或拆分为 `any_return.js`）：
  - class method any return（identity）
  - module function any return（identity）
  - singleton/function any return（identity）

### 4.2 Rust 单测

- 若 generator 侧有单测框架（例如对模板渲染的 snapshot），补一个最小单测：
  - `return any` 的 glue 必须包含 out-callback 路径。

### 4.3 验证命令

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

## 5. 兼容性与非目标

- 不改变 strict 模式对 any 的约束（参见 `docs/planning/2026/v1-strict-any.md`）；本计划仅讨论 default 模式下的 any 语义。
- 不引入 runtime 动态注册（遵守 mquickjs compile-time 注册约束）。

## 6. 风险与回滚策略

- 风险：Any/Local/Env API 不足以在 out 回调中安全构造返回 `JSValue`。
  - 对策：先在 `mquickjs-rs` 补齐必要的“escape/convert”API，并用最小测试验证。

---

> 状态：草案（待你确认后再开始实现）。
