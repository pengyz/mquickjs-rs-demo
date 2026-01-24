<!--
status: 已完成（已落地：ReturnAny + Env::return_safe/pin_return）
owner: Mi Code
tags: mquickjs-rs, ridl, any, api, safety
-->

# 2026-01-24：return-safe any 机制（删 *_out 后的统一兜底方案）

> 目标：
> 1) RIDL glue 删除 `*_out`，用户 trait 直接写 `fn f(...) -> any`；
> 2) 同时保证 **glue 路径** 与 **用户手写 Rust 路径** 对 any 的“跨边界返回”有同一套安全规则；
> 3) 不再依赖“引擎天然可达”作为唯一兜底。

## 0. 背景与现状确认（基于代码）

### 0.1 mquickjs-rs 的值模型

- `Local<'ctx, T>`：绑定 ctx 的 view，**不是 GC root**。
- `Handle<'hs,'ctx,T>`：由 `HandleScope` root，生命周期绑定 `'hs`。
- `Global<T>`：跨 scope 的持久 root。

`Env<'ctx>` 当前内置一个 `HandleScope<'ctx>`：

- `Env::handle(Local) -> Handle`：把 Local 变成 rooted handle（`JS_PushGCRef`）。
- `Env::obj/array/str`：返回 `Handle`（rooted），用于在 Rust 内部安全操作。

### 0.2 当前 any 的 IDL/Rust 映射

- generator filters：`any` => `Local<'ctx, Value>`（参数侧已如此）。
- 但 any-return 目前在模板层被强制走 `*_out`。

我们将删掉 `*_out`，使 any-return 也回到 `Local<'ctx, Value>`。

## 1. 问题：删 *_out 后，“root 保证”从哪来？

- 仅靠“引擎天然可达（返回值自动成为 root）”不够稳：
  - mquickjs 是 tracing GC，root 扫描集合实现细节不透明；
  - 用户也可绕过 RIDL glue，直接用 mquickjs-rs API 写 Rust 逻辑。

因此需要一套 **return-safe** 机制：
- 明确“哪些值允许跨边界返回/存活”；
- 并提供 API 让 glue 与用户手写都能显式地把返回值纳入可达集合。

## 2. 设计目标（必须满足）

1) **用户 API 最简**：用户 trait 方法签名无需 `*_out`、无需暴露 `'hs`。
2) **统一兜底**：
   - RIDL glue 走同一 API；
   - 用户手写 Rust 也能用同一 API 保证 return-safe。
3) **不引入 JSValue 手动 free/dup**（符合引擎 tracing GC 约束）。
4) **对象安全**：继续支持 `dyn Trait`。

## 3. 方案：引入 `ReturnSafe<'ctx, T>`（公共 API）并提供别名 `ReturnAny`

### 3.1 类型形态（建议）

新增一个轻量包装（公共 API）：

```rust
pub struct ReturnSafe<'ctx, T> {
    raw: mquickjs_ffi::JSValue,
    ctx_id: ContextId,
    _marker: PhantomData<Local<'ctx, T>>,
}
```

并提供：

- `impl<'ctx, T> ReturnSafe<'ctx, T> { pub fn as_raw(&self) -> JSValue }`

以及项目内约定的别名（用于“return-safe any”语义）：

```rust
pub type ReturnAny<'ctx> = ReturnSafe<'ctx, Value>;
```

### 3.2 构造方式（用户 & glue 统一）

在 `Env<'ctx>` 增加：

```rust
impl<'ctx> Env<'ctx> {
    pub fn return_safe<T>(&mut self, v: Local<'ctx, T>) -> ReturnSafe<'ctx, T>;
}
```

实现语义：
- 校验 ctx_id 一致
- 通过内部 HandleScope 将 `v` 转成 `Handle` 并 push_gc_ref
- 返回 `ReturnSafe { raw: handle.as_raw(), ... }`

注意：
- `push_gc_ref` 目前是 crate 私有；这里我们通过现有 `Env::handle(v)` 间接调用即可。
- 关键点：ReturnSafe 的 Drop 不做任何事（不 free/dup），仅作为“已被 root（至少在 Env 内）”的证明。

### 3.3 glue 侧使用

当 IDL `-> any`：
- 用户实现返回 `Local<'ctx, Value>`（保持最简）
- glue 在返回 JSValue 前做：

```rust
let result_local: Local<'ctx, Value> = ...;
let ret = env.return_safe(result_local);
return ret.as_raw();
```

这样“root 兜底”不再依赖引擎语义。

### 3.4 用户手写 Rust 的使用方式

用户若自己写 Rust API 并需要跨边界返回/保存：
- 要求使用 `Env::return_safe(local)` 得到 `ReturnSafe<'ctx, T>`；
  - 对于 any：使用别名 `ReturnAny<'ctx>`
- 文档约束：
  - 任何跨越 `Env/Scope` 生命周期的持有必须用 `Global<T>`；
  - 任何“从 Rust 回到 JS 作为返回值”的路径必须先 `env.return_safe(...)`（any 场景可称为 ReturnAny）。

## 4. 兼容性与边界讨论

### 4.1 引擎 pin 机制调研结论（mquickjs）

mquickjs 的 `JS_PushGCRef/JS_PopGCRef` 是一套**通用的临时 root 链**（`JSContext::top_gc_ref`）。

- `JS_PushGCRef(ctx, ref)`：把 `ref` 挂到 `ctx->top_gc_ref` 链头，并返回 `&ref->val` 供写入任意 `JSValue`。
- `JS_PopGCRef(ctx, ref)`：把 `ctx->top_gc_ref` 恢复到 `ref->prev` 并返回 `ref->val`。

证据：`deps/mquickjs/mquickjs.c` 中实现：
- `JSValue *JS_PushGCRef(JSContext *ctx, JSGCRef *ref)`
- `JSValue JS_PopGCRef(JSContext *ctx, JSGCRef *ref)`

mquickjs-rs 的 `HandleScope::handle(Local)`/`push_gc_ref(raw)` 正是对这套机制的封装：
- 先断言 `Local.ctx_id == HandleScope.ctx_id`（禁止 cross-context）
- 再把 `raw = Local.as_raw()` 写入 `JS_PushGCRef` 返回的 slot

因此，从引擎/封装的角度：

- **可以对任意 raw JSValue 进行 pin**（写入 `JSGCRef.val`），引擎层没有“类型限制”；
- **必须满足前置条件：同一个 JSContext**（mquickjs-rs 用 `ctx_id` 断言保证）。

### 4.2 ReturnSafe 的生命周期覆盖问题

在 glue 路径里，如果我们在返回前执行一次 `HandleScope` pin：

- pin 的生命周期至少覆盖到 native function 返回那一刻（因为 `Env/HandleScope` 在 glue 栈上，Drop 发生在返回之后）。

这能作为“过渡窗口”的兜底：
- 在 native 返回值还未被 VM 写入可扫描的位置之前，先确保它在 `top_gc_ref` 链里是可达的。

> 注意：这并不等价于 Global。Global 使用 `JS_AddGCRef`，语义是跨调用持久 root。

### 4.2 何时需要 Global

- 若用户要把 any 存到 struct 里跨调用保存：必须用 `Global<Value>`，ReturnAny 不保证跨调用。

## 5. 实施步骤（建议顺序）

1) 在 mquickjs-rs 增加 `ReturnSafe<'ctx, T>` + `Env::return_safe`（以及别名 ReturnAny）。
2) 先在 tests 里写 Rust-only 用例验证：
   - `return_safe` 后制造大量分配/触发 GC（尽力）仍可取属性。
3) ridl-tool：
   - 删除 `*_out`（rust_api.rs.j2）
   - any-return glue 统一走 `Env::return_safe`。
4) 迁移 tests 下现有 any-return（去掉 *_out 实现）。
5) 全量验证：`cargo test` + `cargo run -- tests`。

## 6. 测试矩阵（必须新增）

- any-return identity：输入对象返回 `===`
- any-return rust-created：返回 string/object/array
- 压力测试：大量分配后仍可访问
- Rust 手写路径：不走 RIDL glue，直接用 Env 构造 Local -> return_safe -> raw -> 再转回 Local 读取

---

> 状态：草案（公共类型：ReturnSafe；any 别名：ReturnAny）。
