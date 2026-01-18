# 计划：mquickjs-rs HandleScope + Escapable（闭包式 API 定稿）（2026-01-18）

> 状态：待确认（确认后才进入实现）

## 0. 背景与结论

我们尝试过“对象式 `EscapableHandleScope` + `escape(self, ...)`”的 V8 直译形态，但在稳定 Rust 的借用规则下会遇到结构性冲突（move/self 消费 vs 借用 lifetime 推导），难以同时满足：
- 易写（常见用法不被 E0505/E0499 卡死）
- 语义明确（一次 escape；未 escape 不能离开）
- 编译期约束（trybuild 可固化）

因此本计划定稿：采用 **闭包承载 inner lifetime（HRTB）** 的 Escapable API 形态。

这是 Rust 社区表达“栈式作用域对象”的惯用模式，能稳定实现并固化 V8 语义。

## 1. 目标与非目标

### 1.1 目标

1) 提供可实现、易用且语义明确的 V8 风格 handle-scope：
- `HandleScope`：外层临时根栈
- `escapable(|inner| ...)`：内层临时根栈（闭包式）
- `escape(...)`：唯一合法的“把值带出 inner scope”的通道

2) 将关键语义变成可测试不变量：
- 未 escape 的 handle 不能离开闭包（compile-fail）
- escape 的 handle 在 outer scope 内可继续使用，并在 GC 后仍可达（运行时测试）

3) 保持项目硬约束（见 AGENTS.md）：
- 编译期注册机制不变
- 不引入硬编码模块名单
- 每项改动配套测试，并跑 `cargo test` + `cargo run -- tests`

### 1.2 非目标

- 本计划不直接实现 `any` 的完整映射；仅提供其需要的 handle/escape 地基。
- 不讨论跨线程、跨 await 的 handle 生命周期（该场景必须 `Global`，见第 4 节）。

## 2. API 形态（定稿建议）

> 术语：
> - `Scope<'ctx>`：context boundary（已有）
> - `Local<'ctx, T>`：非 root 的值视图（已有）
> - `Handle<'hs, 'ctx, T>`：临时 root（新增/改造）
> - `Global<T>`：持久 root（已有）

### 2.1 HandleScope

```rust
pub struct HandleScope<'ctx> { /* holds ctx + root stack */ }

impl<'ctx> HandleScope<'ctx> {
    pub fn new(scope: &'ctx Scope<'ctx>) -> Self;

    /// Root a local value for the lifetime of this handle scope.
    pub fn handle<'hs, T>(&'hs mut self, v: Local<'ctx, T>) -> Handle<'hs, 'ctx, T>;

    /// Create an escapable inner scope.
    ///
    /// The inner scope lifetime `'inner` is bound to the closure body.
    pub fn escapable<R>(
        &mut self,
        f: impl for<'inner> FnOnce(EscapableHandleScope<'inner, 'ctx>) -> R,
    ) -> R;
}
```

关键点：
- `handle(&mut self, ...)` 返回 `Handle<'hs,...>`，`'hs` 绑定到对 `&mut self` 的借用；
- `escapable` 通过 `for<'inner>` 生成真正的 inner lifetime；用户无法把 `Handle<'inner,...>` 直接带出闭包，除非通过 `escape`。

### 2.2 EscapableHandleScope（闭包内可见）

```rust
pub struct EscapableHandleScope<'inner, 'ctx> { /* inner root stack */ }

impl<'inner, 'ctx> EscapableHandleScope<'inner, 'ctx> {
    pub fn handle<T>(&mut self, v: Local<'ctx, T>) -> Handle<'inner, 'ctx, T>;

    /// Escape exactly one handle to the outer scope.
    ///
    /// 语义：消费 self（或 once guard），确保最多 escape 一次。
    pub fn escape<T>(self, v: Handle<'inner, 'ctx, T>) -> Handle<'inner, 'ctx, T>;
}
```

> 注：`escape` 的返回 lifetime 表面上还是 `'inner`，但由于返回值必须作为 `escapable(...)` 的返回值离开闭包，
> 外层 `HandleScope::escapable` 的实现应当把该 escaped 值“转接”为 outer root，并返回 `Handle<'outer,...>` 或 `Handle<'hs,...>`。
> 这里有两种定稿选项（见 2.3）。

### 2.3 escape 的返回类型（两种可选定稿）

**选项 A（推荐）**：`escapable` 直接返回 outer handle

```rust
pub fn escapable<T>(
    &mut self,
    f: impl for<'inner> FnOnce(EscapableHandleScope<'inner, 'ctx>) -> Handle<'inner, 'ctx, T>,
) -> Handle<'_, 'ctx, T>;
```

- inner `escape(...)` 返回 `Handle<'inner,...>`；
- `HandleScope::escapable` 捕获该 raw 值并 push 到 outer root 栈，然后返回 `Handle<'outerBorrow,...>`。

优点：
- 使用者最直觉：`let h_outer = outer.escapable(|inner| inner.escape(h));`
- 返回值在 outer scope 中可继续使用，且类型上不再是 `'inner`。

**选项 B**：`escape` 直接返回 `Global`（不推荐）

- 语义过强、开销更大；不符合 V8 handle 的轻量定位。

本计划建议采用 **选项 A**。

## 3. 使用示例（定稿）

### 3.1 正确：escape 一个值到 outer

```rust
let mut outer = HandleScope::new(&scope);

let escaped = outer.escapable(|mut inner| {
    let v = ctx.create_string(&scope, "x")?;
    let h = inner.handle(v);
    inner.escape(h)
});

// escaped: Handle<'outer, 'ctx, Value>
```

### 3.2 错误：未 escape 直接返回（必须 compile-fail）

```rust
let _bad = outer.escapable(|mut inner| {
    let v = ctx.create_string(&scope, "x")?;
    inner.handle(v) // missing: escape
});
```

## 4. 生命周期边界与使用规则

1) `Handle` 仅在其所属的 handle-scope 生命周期内有效。
2) **跨 await / 跨线程 / 长期保存**：必须升级为 `Global`。
3) `Local` 只是视图：不保证 GC 可达性；任何可能触发 GC 的 native 过程都应通过 `Handle` pin。

## 5. 测试矩阵（实现前先写测试）

### 5.1 trybuild（compile-fail / pass）

- `*.fail.rs`：未 escape 直接从 `escapable(|inner| ...)` 返回 handle，必须编译失败
- `*.pass.rs`：正确 escape 返回 outer handle，必须编译通过

### 5.2 运行时 Rust 单测

1) `escapable_escape_survives_gc`
- outer scope
- inner 创建 object/string -> escape
- inner drop 后 `JS_GC`
- outer 使用 escaped 值仍正确

2) `handle_scope_pins_value_survives_gc`
- outer `handle(local)` pin
- `JS_GC`
- 值仍可用

### 5.3 Repo 根 JS 集成测试

本阶段可选；若需要证明对 glue 的支撑能力，可追加一个最小集成用例。

## 6. 实施步骤（确认后执行）

1) 定稿 2.3：采用选项 A（`escapable` 返回 outer handle）。
2) 先落地 trybuild 测试框架与用例；保证失败/通过用例可稳定运行。
3) 实现 `Handle` / `HandleScope` / `EscapableHandleScope` 的闭包式 API。
4) 补运行时单测，并跑：
   - `cargo test -p mquickjs-rs`
   - `cargo run -- tests`

## 7. 待你确认的问题

1) 2.3 的 escape 返回类型：是否确认选项 A（推荐）？
2) 命名是否确认：`EscapableHandleScope`（与之前偏好一致）？
3) `HandleScope::handle` 命名是否接受（或继续沿用 `create_handle`）？
