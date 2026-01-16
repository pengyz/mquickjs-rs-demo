# V8 风格 Value + Context 体系重构设计（mquickjs）

日期：2026-01-16

> 目标：参考 V8 的 Isolate/HandleScope(Local)/Global(Persistent) 模型，重构 mquickjs-rs 的 Value/Context API，使：
> - 借用值（Local）只能在当前作用域内使用（不可跨作用域保存/返回）。
> - 跨 Rust 边界返回/保存必须使用拥有型句柄（Global）。
> - Context 间完全隔离（等价于 V8 的 Isolate 隔离边界），禁止跨 Context 传值。
> - 与 RIDL glue 的生成模型自然贴合，消除目前围绕 `pin()` / TLS current 的语义与生命周期矛盾。

---

## 0. 约束与背景（本仓库特有）

1. **mquickjs 没有 JSRuntime 概念**：Context 即隔离边界，近似 V8 的 Isolate。
2. **可能存在多个 Context**：不同 Context 完全隔离（不可共享对象/字符串等堆对象）。
3. C API 注册不能运行时动态进行：需要编译期聚合/注册（与本设计无冲突，但要求 glue 生成稳定）。
4. 引擎为 tracing GC：`JSValue` 指向对象/字符串等堆内存由 GC 管理；公开 API 不提供 `JS_FreeValue/JS_DupValue`。

---

## 1. 设计目标与非目标

### 1.1 目标

- 提供清晰的三层抽象：
  - `Context`（隔离边界，≈ V8 Isolate）
  - `Scope`（进入上下文的动态作用域，≈ V8 HandleScope）
  - `Local<T>` / `Global<T>`（值句柄）

- 使 Rust 类型系统能够表达：
  - Local 不能逃逸出 Scope
  - Global 可以跨 Scope / 跨 Rust 边界返回
  - 禁止跨 Context 使用（至少 debug 强校验；尽可能在类型系统上限制）

- RIDL v1 glue 映射：
  - `any` 入参：`Local<Value>`（借用）
  - `any` 返回：`Global<Value>`（拥有/已 root）

### 1.2 非目标

- 不追求与 V8 完全一致的 API 命名/语义（按 Rust + 本工程约束做裁剪）。
- 不在此阶段解决“可在任意线程迁移 Context/Value”的能力（默认 Context 线程亲和）。

---

## 2. 核心概念与类型定义

### 2.1 Context（隔离边界 / Isolate）

建议保留现有 `Context` 作为 owning 类型（负责创建/销毁 JSContext）。

新增：`ContextId`（用于跨 Context 断言）。

```rust
pub struct Context {
    ctx: *mut JSContext,
    id: ContextId,
    // 其他内部资源...
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ContextId(u64);
```

要求：每个 `Context` 生成唯一 `ContextId`。

### 2.2 Scope（HandleScope）

Scope 表示“当前线程已进入某个 Context”的动态作用域。Local 的生命周期绑定到 Scope。

```rust
pub struct Scope<'ctx> {
    ctx: &'ctx Context,
    _guard: EnterGuard<'ctx>,
}

pub struct EnterGuard<'ctx> {
    // RAII：入栈 TLS，出作用域出栈
    _private: (),
}

impl Context {
    pub fn enter(&self) -> Scope<'_> { /* ... */ }
}
```

关键点：
- `Scope<'ctx>` 持有 `&'ctx Context`，因此 `Local<'ctx, T>` 的 `'ctx` 与 Context 绑定。
- Scope 负责 TLS 入栈（仅用于 glue/FFI 回调取“当前 Context”）。

### 2.3 Local（借用句柄）

Local 是 JSValue 的轻量视图，只在 Scope 生命周期内有效。

```rust
pub struct Local<'ctx, T = Value> {
    raw: JSValue,
    ctx_id: ContextId,
    _marker: PhantomData<&'ctx Context>,
    _ty: PhantomData<T>,
}

pub struct Value;
```

构造方式：只能通过 Scope：

```rust
impl<'ctx> Scope<'ctx> {
    pub fn local_value(&self, raw: JSValue) -> Local<'ctx, Value> {
        Local { raw, ctx_id: self.ctx.id, _marker: PhantomData, _ty: PhantomData }
    }
}
```

禁止：对外暴露 `Local::new(raw)` 这类无上下文构造。

### 2.4 Global（拥有句柄 / Persistent）

Global 用于跨 Scope 保存/返回。底层使用 GCRef/root。

```rust
pub struct Global<T = Value> {
    ctx: *mut JSContext,
    ctx_id: ContextId,
    gc_ref: Pin<Box<UnsafeCell<JSGCRef>>>,
    _ty: PhantomData<T>,
}
```

构造：由 Scope 从 Local pin 得到：

```rust
impl<'ctx, T> Local<'ctx, T> {
    pub fn pin(self, scope: &Scope<'ctx>) -> Global<T> { /* JS_AddGCRef(scope.ctx.ctx, ...) */ }
}
```

注意：这里的签名可优化为 `scope.pin(local)`，避免 Local 暴露 pin。

---

## 3. TLS 与“当前 Context”

TLS 仅作为 glue/FFI 回调中取当前 Context 的便利，不作为类型系统的生命周期来源。

建议 TLS 栈元素：

```rust
struct CurrentEntry {
    ctx: *mut JSContext,
    id: ContextId,
}
thread_local! { static TLS_CURRENT: RefCell<Vec<CurrentEntry>> = ...; }
```

- `EnterGuard` push/pop CurrentEntry。
- 提供 `Context::current()` 仅用于 glue 内部（返回 raw ctx + id），不用于构造 Local（Local 必须通过 Scope）。

---

## 4. 跨 Context 规则

- **硬规则**：任何 `Local/Global` 都携带 `ctx_id`，在关键操作（property/get/set/call/pin/convert）做 debug assert：
  - `assert_eq!(value.ctx_id, scope.ctx.id)`
- **不支持**：将 Context A 的 value pin 到 Context B。
- 对 `Global` 的 drop：必须在原 Context 仍存活时执行 JS_DeleteGCRef。
  - 需要 `Context` drop 时先清理所有 Global（可选：在 ContextInner 里维护 weak list；或在 Global drop 时若 ctx 已死则泄露并记录）。
  - 设计权衡：优先保证安全（宁可泄露）还是强制要求 drop 顺序（panic）。文档需明确。

---

## 5. RIDL glue 映射（v1）

### 5.1 参数

- 基础类型（bool/i32/f64/String/...）：按现有规则从 JSValue 转换。
- `any` 入参：生成 `Local<'_, Value>`（或 `Local<'_, Value>` 的别名）传给用户实现。
  - 用户侧若要跨调用保存，必须 `.pin(&scope)` 得到 `Global`。

### 5.2 返回

- `any` 返回：用户实现返回 `Global<Value>`。
- glue 侧：`global.as_raw()` 作为 JSValue 返回。

### 5.3 用户实现签名

- singleton trait：
  - `fn roundtrip_any(&mut self, v: Local<'_, Value>) -> Global<Value>`

---

## 6. API 迁移策略（分阶段）

### Phase 0：冻结/回滚 any

- 现阶段先保证仓库可用：暂不启用 `any` roundtrip（已执行）。

### Phase 1：引入新类型但不替换旧 API

- 新增 `Scope/Local/Global` 与最小工具函数。
- 不改现有 `ValueRef/PinnedValue`，只在新模块中实验。
- 增加单元测试：
  - Local 不能跨 scope 返回（编译期示例，用 doc test）。
  - Global 可以跨 scope 保存并 roundtrip。

### Phase 2：切换 RIDL glue 到新 API

- generator：
  - 在 glue 入口创建 `let scope = ctx.enter();`
  - `any` 参数构造 `scope.local_value(argv[i])`
  - `any` 返回走 `Global::as_raw()`

### Phase 3：弃用旧 ValueRef/PinnedValue

- 标记 deprecated，并提供简单映射：
  - `ValueRef` ≈ `Local<Value>`
  - `PinnedValue` ≈ `Global<Value>`
- 最终删除旧 API。

---

## 7. 测试计划

必须全部通过：

1. `cargo run -p ridl-builder -- prepare`
2. `cargo run -- tests`
3. `cargo test`

新增测试（重构阶段）：
- JS 侧 any roundtrip：number/string/bool/object identity（恢复 `types_full.js` 的相关断言）。
- 跨 Context 负向用例：在 debug 构建触发断言（或返回错误）。

---

## 8. 未决问题（已确认的设计选择）

1. `Global` drop 时 Context 已销毁：选择 **A) panic（严格）**
   - 理由：Context=隔离边界且无 runtime 共享，Global 的生命期必须被显式管理。
   - 落地：`Global::drop` 调用 `JS_DeleteGCRef(self.ctx, ...)` 前先断言 Context 仍存活；
     若已销毁则 `panic!`（提示“Global must be dropped before Context drop”）。

2. 允许同线程嵌套 enter 不同 Context（栈式切换）：**允许**
   - TLS 采用栈，始终以栈顶为 current。
   - `EnterGuard::drop` 必须断言 pop 出的条目与 guard 记录一致，防止乱序 drop：

```rust
impl<'ctx> Drop for EnterGuard<'ctx> {
    fn drop(&mut self) {
        TLS_CURRENT.with(|s| {
            let mut st = s.borrow_mut();
            let top = st.pop().expect("enter/exit stack underflow");
            assert_eq!(top.id, self.expected_id, "Context enter/exit out of order");
            assert_eq!(top.ctx, self.expected_ctx, "Context enter/exit out of order");
        })
    }
}
```

   - 复杂度评估：中等。主要工作量在于：
     - 统一所有需要 ctx 的操作走 `Scope`（或从 `Local/Global` 中携带 ctx_id 并在操作时校验）。
     - 在关键 API（pin/属性访问/函数调用/字符串转换）处添加 `ctx_id` 匹配断言。

3. `Local<Value>` 是否需要 typed wrapper（Local<Object>/Local<String>）？
   - 初期可只实现 `Value`，后续再增加 typed API。
