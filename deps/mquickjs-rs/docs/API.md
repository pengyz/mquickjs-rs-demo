# mquickjs-rs API（v_next）

本文档描述 mquickjs-rs 的公开 API 语义（v_next）：以 V8 风格的 `Scope/Local/Global` 为核心。

## 1. Context / ContextToken

- `Context`：拥有者语义，负责创建/销毁 QuickJS `JSRuntime/JSContext`。
- `ContextToken`：引用语义（轻量可 Clone），用于从 `JSContext*` 反向恢复 Rust 侧上下文，并作为 `Scope` 的入口。

`Context::new()` 会通过 `JS_SetContextUserData` 将 `Arc<ContextInner>` 的 raw 指针写入 JSContext user_data，使得 glue 侧可从 `JSContext*` 反向恢复 `ContextToken`。

常用 API：
- `Context::token() -> ContextToken`
- `unsafe ContextToken::from_js_ctx(ctx: *mut JSContext) -> Option<ContextToken>`
- `ContextToken::current() -> Option<ContextToken>`（仅作为 glue 内部便利，不是生命周期锚点）

## 2. JS 值：Local / Global

mquickjs 的 JSValue 指向对象/字符串等堆内存的生命周期由 tracing GC 管理。

### 2.1 Local<'ctx, T>

- `Local` 是借用型句柄：表示“在某个 `Scope` 内部临时可用”的 JS 值视图。
- `Local` 不能脱离其 `Scope` 的生命周期。
- `T` 是类型标记（编译期约束）：例如 `Local<Value>`、`Local<Object>`、`Local<Function>`。

### 2.2 Global<T>

- `Global` 是可保存型句柄：内部通过 `JSGCRef` 将 JSValue 作为 GC root 持久化。
- `Global::new(&scope, local)`：从 `Local<T>` 创建。
- `reset(&scope, local)` / `reset_empty()`：与 V8 类似，表达“替换/释放持久引用”。

> 注意：严格模式下，如果 `Context` 已经销毁，`Global` 在 drop 时会 panic（用于尽早暴露生命周期错误）。

## 3. Scope

- `Scope` 表示“当前正在进入的 JSContext/上下文边界”。
- `ContextToken::enter_scope() -> Scope` 用于建立该边界。

所有需要调用 QuickJS C API 的操作，原则上都应当在 `Scope` 里完成，并通过 `scope.ctx()` 获取 `JSContext*`。

## 4. 类型化值：Object / Function

- `Local<Value>` 提供 `try_into_object()` / `try_into_function()` 等运行时检查。
- `Local<Object>` / `Local<Function>` 提供更强类型的 API（如 `get_property` / `set_property` / `call`）。

> 这是对 V8 `Local<Object>` / `Local<Function>` 模型的直接映射。
