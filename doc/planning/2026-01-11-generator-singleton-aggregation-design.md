# 设计：ridl-tool 生成 singleton 聚合初始化（方案A：类型擦除槽位）

> 日期：2026-01-11
> 关联计划：`2026-01-11-console-singleton-rtos-plan.md`

## 0. 背景与目标

我们需要在 **RTOS flat mode + 多 JSContext** 下提供 `console` 等 singleton：
- 不使用全局可变单例（避免不同 ctx/task 串台）。
- 不复用 `ctx->opaque`（mquickjs core 已将其用于输出/中断回调）。
- **严格无泄漏**：JSContext 销毁时必须 drop Rust 实例。
- stdlib 静态注册：用户创建 Context 后 singleton 立即可用（无额外 init 手动步骤）。

同时，RIDL 的 singleton trait（例如 `RidlConsoleSingleton`）由声明它的 ridl module 生成。
若聚合层需要引用该 trait，将造成类型依赖扩散与耦合。

**本设计目标**：
- 让聚合层完全不感知任何模块内 trait 类型。
- 让每个模块仍可在自身 crate 内以 trait object 实现业务逻辑。
- 让聚合层负责：创建聚合容器、调用模块 init 填充、绑定 ctx user_data、finalizer 释放。

关键确认（来自讨论结论）：
1) singleton trait 可见性：`pub(crate)`。
2) `CtxExt` 的 slot 命名稳定且与 ridl singleton 名一致：`console -> CtxExt.console`。

---

## 1. 产物分层（生成物清单）

生成器将输出三类产物：

### 1.1 模块内产物（每个 ridl module 一套）

> 这些文件位于 ridl module crate 的 `OUT_DIR` 或其 `generated/` 目录（以既有项目惯例为准），并由该模块的 `lib.rs/mod.rs` include。

- `*_trait.rs`
  - 生成 singleton trait：例如 `pub(crate) trait RidlConsoleSingleton { ... }`
  - trait **不跨 crate 暴露**。

- `*_glue.rs`
  - 生成 mquickjs C ABI glue：`unsafe extern "C" fn js_console_log(...) -> JSValue` 等。
  - glue 通过 `JS_GetContextUserData(ctx)` 拿到 `CtxExt*`，再从 `CtxExt.console` 的类型擦除槽位恢复 `&mut dyn RidlConsoleSingleton` 调用。

- `*_context_init.rs`
  - 生成模块级初始化函数：

    ```rust
    pub fn ridl_module_context_init(ext: &mut CtxExt) {
        // fill CtxExt.console slot
    }
    ```

  - **约束**：只构造/装配 ext 槽位；不触碰 `JS_SetContextUserData`；不调用任何 JS API。

### 1.2 聚合产物（应用层 OUT_DIR 一套）

- `ridl_ctx_ext.rs`
  - 定义 `ErasedSingletonSlot` 与 `CtxExt`。
  - 不出现任何模块 trait 名。

- `ridl_context_init.rs`
  - 定义聚合入口：

    ```rust
    pub unsafe fn ridl_context_init(ctx: *mut JSContext);
    ```

  - 行为：创建 `CtxExt`、调用所有模块 `ridl_module_context_init(&mut ext)`、最后 `JS_SetContextUserData(ctx, ext_ptr, finalizer)`。
  - 提供 finalizer：只做 drop/free（调用 `CtxExt::drop_all()`），不调用 JS API。

### 1.3 应用层 facade（手写，不由 ridl-tool 自动生成）

- `mquickjs-demo/src/context.rs`
  - `Context::new()`：创建 `mquickjs_rs::Context` 后立刻调用 `ridl_context_init(ctx_raw)`。

---

## 2. 核心数据结构（聚合层可见）

### 2.1 类型擦除槽位：`ErasedSingletonSlot`（薄指针）

> 结论更新：我们仍然使用 Rust trait（便于模块内实现与复用），但**不把胖指针直接塞进 `c_void`**。
>
> 方案：把胖指针再包一层 `Box`，让 slot 保存一个薄指针：
>
> - 真实实例：`Box<dyn Trait>` → `*mut dyn Trait`（胖指针值）
> - 再包一层：`Box<*mut dyn Trait>` → `*mut *mut dyn Trait`（薄指针）
> - slot 存 `*mut c_void = holder_ptr as *mut c_void`
>
> 调用时：从 `holder_ptr` 解引用得到原始胖指针，再转为 `&mut dyn Trait`。
> Drop 时：先 drop `Box<dyn Trait>`，再 drop holder 的 `Box<*mut dyn Trait>`。

```rust
use core::ffi::c_void;

pub struct ErasedSingletonSlot {
    ptr: *mut c_void,
    drop_fn: Option<unsafe fn(*mut c_void)>,
}

impl ErasedSingletonSlot {
    pub const fn empty() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            drop_fn: None,
        }
    }

    pub fn is_set(&self) -> bool {
        !self.ptr.is_null()
    }

    pub fn ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Safety: caller must ensure (ptr, drop_fn) match the allocation.
    pub unsafe fn set(&mut self, ptr: *mut c_void, drop_fn: unsafe fn(*mut c_void)) {
        debug_assert!(self.ptr.is_null());
        self.ptr = ptr;
        self.drop_fn = Some(drop_fn);
    }

    /// Safety: may only be called once.
    pub unsafe fn drop_in_place(&mut self) {
        if let Some(f) = self.drop_fn.take() {
            let p = core::mem::replace(&mut self.ptr, core::ptr::null_mut());
            if !p.is_null() {
                f(p);
            }
        }
    }
}
```

### 2.2 per-context 容器：`CtxExt`

- slot 字段名与 ridl singleton 名一致。

```rust
pub struct CtxExt {
    pub console: ErasedSingletonSlot,
    // future: pub kv: ErasedSingletonSlot, ...
}

impl CtxExt {
    pub const fn new() -> Self {
        Self { console: ErasedSingletonSlot::empty() }
    }

    /// Safety: called from JSContext finalizer; MUST NOT call any JS API.
    pub unsafe fn drop_all(&mut self) {
        self.console.drop_in_place();
        // future slots...
    }
}
```

---

## 3. 模块内 init 如何填充 slot（胖指针外包一层）

以 stdlib 的 `console` 为例：

- 模块内部仍然用 trait 组织实现：`pub(crate) trait ConsoleSingleton { ... }`
- 但 slot 存的是 **`Box<*mut dyn ConsoleSingleton>` 的 raw 指针**（薄指针），从而绕开“胖指针不能塞进 `c_void`”的问题。

模板化伪码：

```rust
use core::ffi::c_void;

pub(crate) trait ConsoleSingleton {
    fn log(&mut self, ctx: *mut JSContext, args: Vec<JSValue>);
    fn error(&mut self, ctx: *mut JSContext, args: Vec<JSValue>);
    fn enabled(&self) -> bool;
}

pub(crate) struct DefaultConsoleSingleton;

impl ConsoleSingleton for DefaultConsoleSingleton {
    /* ... */
}

unsafe fn console_drop(holder_ptr: *mut c_void) {
    // holder owns a copy of the fat pointer value
    let holder: Box<*mut dyn ConsoleSingleton> = Box::from_raw(holder_ptr as *mut *mut dyn ConsoleSingleton);
    // first drop the real trait object
    drop(Box::from_raw(*holder));
    // then drop holder itself (by dropping `holder`)
}

pub fn ridl_module_context_init(ext: &mut CtxExt) {
    let obj: Box<dyn ConsoleSingleton> = Box::new(DefaultConsoleSingleton);
    let fat: *mut dyn ConsoleSingleton = Box::into_raw(obj);
    let holder: Box<*mut dyn ConsoleSingleton> = Box::new(fat);
    let holder_ptr: *mut *mut dyn ConsoleSingleton = Box::into_raw(holder);

    unsafe {
        ext.console.set(holder_ptr as *mut c_void, console_drop);
    }
}
```

约束：
- 不允许在模块 init 中调用 JS API。
- 不允许在模块 init 中 set ctx user_data。

---

## 4. glue 如何访问 slot 并转发调用（解引用 holder 得到胖指针）

glue 通过 slot 的 `void*` 取回 `holder_ptr: *mut *mut dyn Trait`，再解引用得到 `*mut dyn Trait`：

```rust
fn console_mut(ext: &mut CtxExt) -> &mut dyn ConsoleSingleton {
    if !ext.console.is_set() {
        // throw TypeError in caller
        unreachable!();
    }
    let holder_ptr = ext.console.ptr() as *mut *mut dyn ConsoleSingleton;
    let fat = unsafe { *holder_ptr };
    unsafe { &mut *fat }
}

pub unsafe extern "C" fn js_console_log(ctx: *mut JSContext, this_val: JSValue, argc: c_int, argv: *mut JSValue) -> JSValue {
    let mut h = ContextHandle::from_js_ctx(ctx).ok_or_throw_type_error()?;
    let ext = h.ridl_ext_mut().ok_or_throw_type_error()?;

    let s = console_mut(ext);
    let args = slice_to_vec(argv, argc);
    s.log(ctx, args);
    JS_UNDEFINED
}
```

错误策略（模板必须统一）：
- `ContextHandle::from_js_ctx(ctx)` 失败：抛 TypeError（Context 未初始化/非本库创建）
- ridl_ext 未初始化：抛 TypeError（应用未调用 `ridl_context_init`）
- slot 未 set：抛 TypeError（模块 init 未填充/未被聚合调用）

---

## 5. A2：ctx user_data = `Arc<ContextInner>` 与 `ContextHandle::from_js_ctx`

为了让 Rust 侧 API 不暴露 `mquickjs_ffi`，并允许在任何拿到 `JSContext*` 的位置恢复宿主 `Context` 能力：

- ctx user_data **不再存放 `CtxExt`**。
- ctx user_data 存放 `Arc<ContextInner>`（或等价稳定地址的 inner/ext 结构）。
- `ContextInner` 内部包含 RIDL 扩展：`ridl_ext: ridl::CtxExt`。
- RIDL 聚合初始化 `ridl_context_init(ctx)` 的职责变为：
  - 通过 `ContextHandle::from_js_ctx(ctx)` 拿到 `&mut ContextInner`
  - 初始化/填充 `ContextInner.ridl_ext` 的各个 slot（调用各模块 `ridl_module_context_init(&mut ridl_ext)`）
  - **不再调用 `JS_SetContextUserData`**（这由 `mquickjs_rs::Context::new()` 完成）

### 5.1 `ContextInner` / `Context` / `ContextHandle`

建议形态：

```rust
pub struct Context {
    ctx: *mut JSContext,
    inner: std::sync::Arc<ContextInner>,
}

pub struct ContextInner {
    pub ridl_ext: ridl::CtxExt,
    // future: host IO handles / metrics / config ...
}

/// Borrow-like handle reconstructed from JSContext.
/// It must NOT free the JSContext.
pub struct ContextHandle {
    ctx: *mut JSContext,
    inner: std::sync::Arc<ContextInner>,
}

impl ContextHandle {
    /// Safety: ctx must be alive; user_data must be a valid Arc<ContextInner>.
    pub unsafe fn from_js_ctx(ctx: *mut JSContext) -> Option<Self> {
        let p = JS_GetContextUserData(ctx);
        if p.is_null() { return None; }
        // p is *const Arc<ContextInner> or raw Arc ptr depending on storage strategy.
        // Reconstruct an Arc clone, leaving the original for Context ownership.
        Some(Self { ctx, inner: arc_clone_from_user_data(p) })
    }

    pub fn ridl_ext_mut(&mut self) -> &mut ridl::CtxExt {
        // requires interior mutability if ContextHandle is shared; see note below.
        &mut self.inner.ridl_ext
    }
}
```

> 注意：`Arc<ContextInner>` 默认只提供共享引用；若需要从 glue 中拿到 `&mut ridl_ext`，`ContextInner` 内部应使用 `Mutex/RefCell/UnsafeCell` 之类的内部可变性方案（具体选型需结合本项目并发模型；RTOS 下通常由外部保证同一 ctx 不并发使用）。

### 5.2 user_data 存储与释放顺序（Drop 负责释放）

- `Context::new()`：
  - 创建 `Arc<ContextInner>`
  - 将 **一份 Arc 的克隆**放入 ctx user_data，并注册 finalizer：finalizer 仅 drop 这一份 Arc

- `Drop for Context`：
  - 先调用 `JS_FreeContext(ctx)`（触发 finalizer，drop user_data 里的 Arc clone）
  - 再 drop `Context.inner`（drop Context 自己持有的 Arc）

这样可保证：
- `JS_FreeContext` 阶段 user_data 有效
- 不会 double-free

---

## 6. RIDL 聚合初始化入口（在 A2 下的职责）

生成器输出 `ridl_context_init.rs`：

```rust
pub unsafe fn ridl_context_init(ctx: *mut JSContext) {
    // 1) reconstruct host handle
    let mut h = ContextHandle::from_js_ctx(ctx).ok_or_throw_type_error()?;

    // 2) idempotent init of ridl_ext slots
    let ext = h.ridl_ext_mut();
    stdlib::ridl_module_context_init(ext);
    // other modules...
}
```

约束：
- 仍然要求模块 init 不调用 JS API。
- `ridl_context_init` 不触碰 user_data set/finalizer（由 mquickjs-rs Context 负责）。

---

## 6. ridl-tool 模板与渲染输入（Plan）

### 6.1 Plan 最小字段

- `modules[]`：
  - `crate_name`（用于生成 `stdlib::ridl_module_context_init` 路径）
  - `singletons[]`：
    - `name`（例如 `console`，用于 slot 名）
    - `methods[]` / `properties[]`（用于 trait + glue）

### 6.2 模板清单（建议命名）

- `templates/rust_singleton_trait.rs.j2`
- `templates/rust_singleton_glue.rs.j2`
- `templates/rust_module_context_init.rs.j2`
- `templates/rust_ctx_ext.rs.j2`
- `templates/rust_context_init_aggregated.rs.j2`

### 6.3 关键渲染规则

- trait 生成：`pub(crate)`
- slot 字段名：与 singleton 名一致；若 ridl 名非法标识符，需有统一的 `sanitize_ident()` 规则（例如 `console` 原样）。
- `CtxExt::drop_all`：对所有 slot 调用 `drop_in_place`，顺序稳定（按 singleton 名排序）。

---

## 7. 待确认点（进入实现前）

1) `CtxExt` 的定义位置：
   - 建议：聚合生成物 `ridl_ctx_ext.rs`，由应用层 include。
   - 模块侧通过 `use crate::...::CtxExt` 引用该类型。

2) 模块侧 `DefaultConsole::new()` 的提供位置：
   - stdlib 模块手写默认实现；生成器只负责调用约定的构造函数，或生成一个 `Default...` 占位类型由 stdlib 实现。

3) slot overwrite 策略：
   - 暂定：禁止覆盖（debug_assert empty）。若未来要支持注入自定义实现，则需要 builder 在调用 `ridl_context_init` 前配置，或允许模块 init 检测已 set 则不覆盖。
