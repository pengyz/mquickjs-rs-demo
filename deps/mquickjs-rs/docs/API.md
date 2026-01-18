# mquickjs-rs API（v_next）

本文档描述 mquickjs-rs 的公开 API 语义（v_next）：以 V8 风格的 `Scope/Local/Handle/Global` 为核心。

## 1. Context / ContextToken

- `Context`：拥有者语义，负责创建/销毁 QuickJS `JSRuntime/JSContext`。
- `ContextToken`：引用语义（轻量可 Clone），用于从 `JSContext*` 反向恢复 Rust 侧上下文，并作为 `Scope` 的入口。

`Context::new()` 会通过 `JS_SetContextUserData` 将 `Arc<ContextInner>` 的 raw 指针写入 JSContext user_data，使得 glue 侧可从 `JSContext*` 反向恢复 `ContextToken`。

常用 API：
- `Context::token() -> ContextToken`
- `unsafe ContextToken::from_js_ctx(ctx: *mut JSContext) -> Option<ContextToken>`
- `ContextToken::current() -> Option<ContextToken>`（仅作为 glue 内部便利，不是生命周期锚点）

## 2. JS 值：Local / Handle / Any / Global

mquickjs 的 JSValue 指向对象/字符串等堆内存的生命周期由 tracing GC 管理。

### 2.1 Local<'ctx, T>

- `Local` 是借用型句柄：表示“在某个 `Scope` 内部临时可用”的 JS 值视图。
- `Local` 不能脱离其 `Scope` 的生命周期。
- `T` 是类型标记（编译期约束）：例如 `Local<Value>`、`Local<Object>`、`Local<Function>`。

### 2.2 Handle<'hs, 'ctx, T>

- `Handle` 是“临时 GC root”：通过 `HandleScope` / `EscapableHandleScope` 在当前 native 执行片段内 pin 住 JSValue。
- `Handle` 的生命周期由 handle-scope 约束：离开对应作用域后不可再使用（编译期保证）。
- `Handle` 适用于 glue/native 代码内部的临时值传递；需要跨更长生命周期时请使用 `Global`。

#### 2.2.1 HandleScope / EscapableHandleScope（闭包式）

- `HandleScope::handle(&mut self, local) -> Handle`：将一个 `Local` 作为临时 root pin。
- `HandleScope::escapable(|inner| ...) -> Handle`：创建内层作用域，闭包结束自动释放 inner roots；
  仅能通过 `inner.escape(...)` 把一个值带出。

示例：

```rust
let token = ctx.token();
let scope = token.enter_scope();

let mut hs = HandleScope::new(&scope);
let scope_ref = hs.scope();

let escaped = hs.escapable(|mut inner| {
    let v = ctx.create_string(scope_ref, "x").unwrap();
    let h = inner.handle(v);
    inner.escape(h)
});
```

### 2.3 Any<'hs, 'ctx>

- `Any` 是 `Value` 的语义特化（newtype）：表达“业务层动态值”。
- 当前实现中，`Any` 内部持有 `Handle<'hs,'ctx,Value>`，因此它在所在 `HandleScope` 生命周期内是 GC-safe 的。
- RIDL glue/native 代码都推荐用 `Any` 承载 `any` 参数与动态返回值。

### 2.4 Global<T>

- `Global` 是可保存型句柄：内部通过 `JSGCRef` 将 JSValue 作为 GC root 持久化。
- `Global::new(&scope, local)`：从 `Local<T>` 创建。
- `reset(&scope, local)` / `reset_empty()`：与 V8 类似，表达“替换/释放持久引用”。

> 注意：严格模式下，如果 `Context` 已经销毁，`Global` 在 drop 时会 panic（用于尽早暴露生命周期错误）。

## 3. Scope

- `Scope` 是 context boundary，不负责临时 root；临时 root 由 `HandleScope` 管理。
- 建议：所有可能触发 GC 的 native 过程，应使用 `HandleScope`/`Handle` pin 住需要保持可达的值。


- `Scope` 表示“当前正在进入的 JSContext/上下文边界”。
- `ContextToken::enter_scope() -> Scope` 用于建立该边界。

所有需要调用 QuickJS C API 的操作，原则上都应当在 `Scope` 里完成，并通过 `scope.ctx()` 获取 `JSContext*`。

## 4. 类型化值：Object / Function

- `Local<Value>` 提供 `try_into_object()` / `try_into_function()` 等运行时检查。
- `Local<Object>` / `Local<Function>` 提供更强类型的 API（如 `get_property` / `set_property` / `call`）。

> 这是对 V8 `Local<Object>` / `Local<Function>` 模型的直接映射。
