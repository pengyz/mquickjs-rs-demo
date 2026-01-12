# mquickjs-rs：设计与 API 说明

本文档描述 mquickjs-rs 的核心 API 形态与设计约束，尤其是：
- Context/ContextHandle 的分层（拥有者 vs 引用语义）
- JS 值的两层语义：ValueRef（借用）与 PinnedValue（GCRef pin/unpin）
- Thread-Local current Context（TLS current）

> 重要：mquickjs 的接口与 QuickJS 不同；本项目禁止按 QuickJS 运行时注册模型设计。

## 1. Context / ContextHandle

### 1.1 Context（拥有者）
`Context` 负责创建/销毁底层 `JSContext*`，并持有用于 JS 堆的内存块。其 Drop 会调用 `JS_FreeContext`。

- `Context` 不应被 Clone。
- `Context` 内部持有一个 `Arc<ContextInner>`，用于承载 per-JSContext 扩展（ridl_ext）。
- `Context::new()` 会通过 `JS_SetContextUserData` 将 `Arc<ContextInner>` 的 raw 指针写入 JSContext user_data，使得 glue 侧可从 `JSContext*` 反向恢复 `ContextHandle`。

### 1.2 ContextHandle（引用语义）
`ContextHandle` 是轻量可 Clone 的句柄：
- 不拥有 JSContext（不会 free）
- 持有 `ctx: *mut JSContext` 与 `inner: Arc<ContextInner>`
- 可通过 `unsafe fn from_js_ctx(ctx: *mut JSContext)` 从 user_data 重建

用途：
- glue 层在只拿到 `JSContext*` 的情况下，恢复 Rust 侧的 per-context 扩展（ridl_ext）。
- 作为 TLS current 的存储类型（见第 3 节）。

### 1.3 线程模型
mquickjs 的 JSContext 语义为单线程；因此 Context/ContextHandle/ValueRef/PinnedValue 应避免跨线程传递。

- 不推荐为 ContextInner 实现 Send/Sync。
- 如确需多线程，必须由上层做线程隔离（每线程独立 Context）。

## 2. JS 值：ValueRef / PinnedValue

mquickjs 的 JSValue **不是引用计数模型**，无法通过 clone/dup/free 获得“拥有语义”。
其生命周期管理依赖 GC root 机制（JSGCRef）：
- 栈式：`JS_PushGCRef` / `JS_PopGCRef`（宏 `JS_PUSH_VALUE` / `JS_POP_VALUE`）
- 列表式：`JS_AddGCRef` / `JS_DeleteGCRef`

因此 mquickjs-rs 采用两层值语义：

### 2.1 ValueRef<'ctx>（借用视图）
- 仅在当前调用栈内使用（例如从 argv/this 读取、做一次转换/调用）。
- **禁止**长期保存（例如放入 struct 字段跨调用复用），否则 GC 可能导致悬垂。
- 典型 API：类型检查、读写属性、必要时转换为 PinnedValue。

示例：
```rust
fn f(ctx: &Context, v: ValueRef<'_>) -> Result<String, String> {
    if v.is_string(ctx) {
        ctx.get_string(v)
    } else {
        Err("not string".into())
    }
}
```

### 2.2 PinnedValue<'ctx>（可保存值）
- 通过 `JS_AddGCRef` 把值加入 ctx 的 GC root 列表（pin），从而允许跨调用/跨 GC 存活。
- Drop 时调用 `JS_DeleteGCRef`（unpin）。
- 生命周期必须绑定同一个 Context（不得跨 ctx）。

示例：
```rust
let v = ctx.create_string("token")?;
let pinned = v.pin(&ctx);
unsafe { mquickjs_ffi::JS_GC(ctx.ctx) };
assert_eq!(ctx.get_string(pinned.as_ref())?, "token");
```

## 3. TLS current Context（可选辅助）

为减少 glue/impl 层参数穿透，可提供 per-thread 的 current ContextHandle：

- `ContextHandle::current() -> Option<ContextHandle>`
- `ContextHandle::enter_current(&self) -> CurrentGuard`（RAII，支持嵌套恢复）
- `ContextHandle::with_current(&self, f: impl FnOnce() -> R) -> R`

约束：
- current 只用于“便捷获取”上下文，不替代生命周期约束（ValueRef/PinnedValue 仍需绑定 ctx）。
- finalizer 路径禁止调用 JS API：即使能拿到 current ctx，也不可在 finalizer 内做 JS 操作。

推荐使用方式：
- 在进入 JS 执行/回调边界时设置 current（例如 Context::eval/调用 glue 入口）。
- 通过 guard 自动恢复上一层 current，避免嵌套调用污染。

## 4. ridl_ext（per-Context 扩展）

`ContextInner` 提供 type-erased 的 ridl_ext 插槽：
- 由应用生成的 `ridl_context_init(ctx)` 初始化
- Drop 时仅执行 drop_fn（不得调用 JS API）

这为 ridl singleton / 未来 proto property（per-ctx 共享态）提供承载位置。
