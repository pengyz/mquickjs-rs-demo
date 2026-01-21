<!-- planning-meta
status: 未复核
tags: context-init, engine, handlescope, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `handlescope` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 计划：mquickjs-rs HandleScope + EscapableScope（前置：any/GC root 机制）（2026-01-18）

> 状态：草案（待你确认后才能进入实现）

## 0. 背景

在 `docs/planning/2026/2026-01-17-fn覆盖与HandleScope前置-状态快照.md` 中已确认：
- fn 覆盖推进过程中被 `any` 的 Rust 映射与 mquickjs 的 GC root/可达性机制阻塞；
- 必须先补齐机制型 `HandleScope`（对外 API），再回到 fn 覆盖与 strict 行为。

当前代码现状（已存在部分实现）：
- `Scope<'ctx>`：进入 JSContext 的边界（TLS enter/exit 栈 + ContextId），用于调用 C API。
- `Local<'ctx, T>`：借用视图（仅携带 JSValue + ctx_id + PhantomData），本身不是 GC root。
- `Global<T>`：持久 GC root（`JS_AddGCRef/JS_DeleteGCRef`），并强制约束 **必须在 Context drop 前 drop**（否则 panic）。
- `HandleScope<'ctx>`：临时 GC root（`JS_PushGCRef/JS_PopGCRef`，链入 `ctx->top_gc_ref`），但目前缺少“逃逸/返回”的显式机制。

我们需要按 V8 风格补齐：
- `HandleScope`：用于在 native 代码执行期间临时 pin 住若干 `Local`，确保中途发生 GC 也不会把 JS 对象回收。
- `EscapableScope`：允许从当前 scope “逃逸”一个 handle，供上层 scope 使用；其余 handle 在当前 scope drop 时失效。

## 1. 目标与非目标

### 1.1 目标（本阶段验收口径）

1) 提供对外可用的 V8 风格 API：
   - `HandleScope`：创建、在其生命周期内创建临时 handle。
   - `EscapableScope`：允许 escape 一个 handle 到外层 `HandleScope`。

2) 语义必须可被测试验证：
   - 未 escape 的值：在 inner scope drop 后不可再被安全使用（至少在 API 层无法编译/无法构造）。
   - escape 的值：可以在 outer scope 内继续使用；并在 GC 后仍保持可达。

3) 保持项目硬约束：
   - 不引入运行时 C API 注册。
   - 不引入硬编码模块名单。

### 1.2 非目标（明确不在本计划范围）

- 不在本计划中实现 RIDL glue 对 `any` 的完整映射；本计划只提供能支撑该映射的 scope 机制。
- 不在本计划中实现 class/opaque 的生命周期机制。

## 2. 术语与 API 草案（待你确认）

> 注意：当前 crate 已经暴露 `HandleScope`，但没有 `EscapableScope`。

### 2.1 建议新增类型

- `pub struct EscapableHandleScope<'outer, 'ctx>`（名字可调整）：
  - 绑定外层 `HandleScope`（或外层 `Scope`）以承载 escape 出去的临时 root。
  - 只允许调用一次 `escape(...)`（匹配 V8），否则 panic 或返回 Err（需要你确认）。

### 2.2 建议的最小 API 形状

- `impl<'ctx> HandleScope<'ctx> {`
  - `pub fn new(scope: &'ctx Scope<'ctx>) -> Self`（已存在）
  - `pub fn create_handle<T>(&self, v: Local<'ctx, T>) -> Local<'ctx, T>`（需要泛型化；当前只接受 `Local<Value>`）
  - `pub fn escapable<'outer>(&'outer self) -> EscapableHandleScope<'outer, 'ctx>`（新增）
  `}`

- `impl<'outer, 'ctx> EscapableHandleScope<'outer, 'ctx> {`
  - `pub fn new(outer: &'outer HandleScope<'ctx>) -> Self`（或私有构造）
  - `pub fn create_handle<T>(&self, v: Local<'ctx, T>) -> Local<'ctx, T>`（同上，inner 临时 root）
  - `pub fn escape<T>(self, v: Local<'ctx, T>) -> Local<'ctx, T>`（把 v 变成 outer 的临时 root 并返回）
  `}`

关键点：
- “返回给外层”的 `Local` 仍是 `Local<'ctx, T>`，其可用性靠生命周期约束（`'ctx`）+ API 构造路径保证。
- 逃逸的本质是：把 JSValue 追加到 outer 的 `ctx->top_gc_ref` 链（outer 自己的 head 维护），使其在 outer drop 前可达。

> 需要你确认：EscapableScope 是否应该同时引入独立的 `'inner` 生命周期来表达“inner scope 创建的 Local 不能直接返回”？
> 目前代码里 `Local` 的 lifetime 是 `Scope<'ctx>` 的 `'ctx`，没有 inner/outer 区分；因此仅靠 Rust 生命周期可能无法表达“inner 的 handle 不可返回”。
> 这可能需要：
> - `Local<'a, T>` 绑定到 `HandleScope<'a>` 而不是 `Scope<'ctx>`；或
> - 引入 `Handle<'hs, 'ctx, T>` 两维参数；或
> - 保持 `Local` 表示“值视图”，并用 `HandleScope::create_handle` 返回一种新类型 `Handle<'hs, T>`。

本计划的核心是先把该点讨论定下来。

## 3. 关键设计决策点（需要你拍板）

### 3.1 Local 的 lifetime 应该绑定到谁？

现状：`Local<'ctx, T>` 只绑定 `Scope<'ctx>` 的 `'ctx`，导致：
- 理论上 inner/outer HandleScope 都能返回 `Local<'ctx, T>`，Rust 类型系统无法禁止“跨 HandleScope 返回”。

选项 A（更接近 V8 的类型系统约束）：
- 新增 `pub struct Handle<'hs, T>`（或 `Local<'hs, T>` 重定义）把 lifetime 绑定到 `HandleScope`，从而 inner scope drop 后无法使用。
- `Scope` 仅用于提供 `ctx()` 与 context_id，不负责 handle 生命周期。

选项 B（保持现状，靠运行时规则/文档约束）：
- 继续使用 `Local<'ctx, T>`，并声明：只有通过 `HandleScope`/`Global` 产生的值在 GC 下安全；
- 但无法在编译期阻止误用，风险更高。

### 3.2 EscapableScope 的 escape 约束

- 只允许 escape 一次：
  - V8 是 compile-time/structure 上保证（C++ API），Rust 这里可选择：
    - 消费 self 的 `escape(self, ...)`（天然一次）；或
    - `&mut self` + 内部标志位。

### 3.3 与 Global 的关系

- `Global` 是持久 root，且必须在 Context drop 前 drop（当前实现会 panic）。
- `EscapableScope` escape 的值应当是临时 root（outer HandleScope 生命周期内），不应升级为 Global。

## 4. 测试矩阵（先写测试再实现）

### 4.1 Rust 单元测试（mquickjs-rs crate 内）

新增/调整测试点：

1) `handle_scope_pins_value_survives_gc`
- 步骤：
  - `Context::new` + `token.enter_scope()`
  - 创建 object/string，放入 HandleScope 的临时 root
  - 手动 `JS_GC(ctx)`
  - 读取值仍可用（例如 string roundtrip / object property）

2) `escapable_scope_escape_survives_after_inner_drop`
- 结构：
  - outer `HandleScope`
  - inner `Escapable...` 创建值并 `escape`
  - inner drop 后，触发 GC
  - 仍可从 outer 使用该值

3) `escapable_scope_non_escaped_dies_at_compile_time_or_is_unreachable`
- 若我们选择选项 A（让 handle 绑定到 HandleScope lifetime），则可用 **compile-fail** 测试（需要引入 trybuild；需先确认仓库是否已有该依赖/惯例）。
- 若选择选项 B，则只能做运行时约束（不推荐，但可讨论）。

### 4.2 JS 集成测试（repo 根）

本计划原则上不直接改 RIDL/JS 用例；但为了证明“any 前置能力已具备”，可新增一个最小 JS 用例：
- 在 native 侧通过 API 暴露一个函数：创建对象并在中途触发 GC，再返回对象/字符串给 JS。
- 该部分需要先确认：目前 repo 是否已有用于测试 Rust API 的入口（可能通过 existing global modules）。

## 5. 实施步骤（在你确认方案后执行）

1) 先定稿：Local/Handle 的 lifetime 绑定方案（第 3.1 节）。
2) 基于定稿写测试（优先 Rust 单测；若需要 compile-fail 再讨论工具链）。
3) 实现 HandleScope/EscapableScope（或替换现有 HandleScope 实现）并跑：
   - `cargo test`
   - `cargo run -- tests`

## 6. 待你确认的问题（必须答复）

1) 你希望采用 3.1 的哪种方案？
   - A：引入绑定到 HandleScope lifetime 的 handle 类型（更安全）
   - B：保持现状 Local<'ctx>，靠文档/约定（实现更快但不够安全）

2) EscapableScope 的 API 命名偏好：
   - `EscapableHandleScope` vs `EscapableScope` vs `EscapeScope`？

3) 是否允许为 compile-fail 引入测试依赖（如 `trybuild`）？如果不允许，我们就只能做运行时测试。
