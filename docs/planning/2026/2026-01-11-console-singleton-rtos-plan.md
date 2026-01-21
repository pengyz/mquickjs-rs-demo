<!-- planning-meta
status: 未复核
tags: context-init, engine, ridl, types
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
-->

> 状态：**未复核**（`context-init` `engine` `ridl` `types`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
>
> 关键结论：
> - （待补充：3~5 条）
# 方案：RTOS flat + 多 JSContext 隔离的 console singleton（JS_OBJECT_DEF + ctx user_data + FreeContext finalizer，无泄漏）

> 日期：2026-01-11

## 目标约束汇总

- mquickjs 只有 JSContext，无 JSRuntime。
- RTOS flat mode：全局变量跨 task 不隔离，不能用全局单例保存可变状态（否则多 ctx 串台）。
- stdlib 静态注册：console 对象创建后应立即可用，不希望额外 init 步骤（或尽量内聚到 `Context::new`）。
- `JS_OBJECT_DEF`：只能定义对象名+属性列表；无 class_id/constructor/finalizer/opaque（对象级）。
- 严格无泄漏：必须可在 JSContext 销毁时释放 Rust 实例。
- 接口建模：采用 `singleton console { ... }`；生成 `ConsoleSingleton` trait。
- `console.enabled` 是 getter；`log/error` 是 varargs any；行为由实现层决定，与 strict 无关。

---

## 1) RIDL 语法与语义

### 1.1 语法

```ridl
singleton console {
  fn log(...args: any);
  fn error(...args: any);
  readonly property enabled: bool;
}
```

### 1.2 语义（关键）

- `singleton` 表示“某个全局对象名”的接口建模，不承诺 class 实例/this->opaque。
- Rust 实现通过 “per-JSContext 实例” 提供（避免全局共享）。
- JS 对象本身仍由 stdlib 注册产生（`JS_OBJECT_DEF`），成员函数/属性由 glue 实现并通过 `ctx` 取实例。

---

## 2) Rust 侧接口（trait）与 ctx user_data 容器

### 2.1 生成 trait：ConsoleSingleton

生成到 stdlib 模块 crate（或对应 ridl-module crate）：

```rust
pub trait ConsoleSingleton {
    fn log(&mut self, ctx: *mut JSContext, args: &[JSValue]);
    fn error(&mut self, ctx: *mut JSContext, args: &[JSValue]);
    fn enabled(&mut self, ctx: *mut JSContext) -> bool;
}
```

### 2.2 per-context user_data（CtxExt）

把“所有 ridl singleton 的 per-ctx 实例”统一聚合到一个结构里，作为 ctx user_data：

```rust
pub struct CtxExt {
    pub console: Box<dyn ConsoleSingleton>,
    // 未来可扩展：其他 singleton 的实例也放这里
}
```

### 2.3 所有权与泄漏规避（由引擎 FreeContext 回调释放）

本方案**不依赖 Rust `Context` wrapper 的 Drop 顺序**来释放 user_data，而是把释放绑定到 `JS_FreeContext`：

- mquickjs 新增 ctx user_data 槽位（独立于 `ctx->opaque`）。
- 通过 `JS_SetContextUserData(ctx, user_data, finalizer)` 设置指针与 finalizer。
- 在 `JS_FreeContext` 内部调用该 finalizer，以释放 Rust 实例。

约束：finalizer **只允许释放内存（drop）**，不允许做任何 JS API 操作。

---

## 3) ctx->opaque 绑定与访问

### 3.1 绑定时机（无感初始化）

目标：用户只需创建 `Context::new()`，console 等 ridl 扩展**自动可用**；新增 ridl 模块也不要求用户手动逐个初始化。

做法：把扩展初始化内聚为一个“聚合初始化函数”，并由 `Context::new()` 固定调用一次。

- 创建 JSContext（`JS_NewContext`）
- 调用聚合初始化：`ridl_bootstrap(ctx)`
  - 该函数由 build.rs + ridl-tool 在构建期聚合生成
  - 在内部创建 `CtxExt` 并调用 `JS_SetContextUserData(ctx, ext_ptr, finalizer)`

这样用户拿到 Context 后 console 已可用，且扩展数量不会冲击 mquickjs-rs 的公开 API。

### 3.2 访问方式（在 glue 里）

#### 3.2.1 现状盘点：ctx->opaque 目前被 mquickjs 内部使用

在当前仓库版本中，mquickjs 内部已经把 `ctx->opaque` 当作 **输出/中断回调的 opaque** 使用：

- debug/print 输出路径：
  - `js_vprintf(ctx->write_func, ctx->opaque, ...)`
  - `ctx->write_func(ctx->opaque, ...)`
- 中断回调路径：
  - `ctx->interrupt_handler(ctx, ctx->opaque)`

因此，`ctx->opaque` 并不是“留给外部扩展任意占用的 user_data 槽位”。如果我们把它改成 `CtxExt*`，则：

- write_func 的 opaque 将变成 `CtxExt*`（除非我们也把 write_func 设计为能接收 `CtxExt*` 并从中找到真正的输出目标）
- interrupt_handler 的 opaque 同样会变成 `CtxExt*`（同理）

这会引入耦合与潜在功能性问题。

#### 3.2.2 建议：新增专用 user_data slot（不复用 ctx->opaque）

为了同时满足：
- stdlib 静态注册下的 per-ctx 隔离
- 不破坏 mquickjs 内部对 `ctx->opaque` 的既有用途
- 严格无泄漏

推荐在 mquickjs 增加一套“扩展专用”的 ctx user_data API（独立字段）：

- `typedef void (*JSContextUserDataFinalizer)(JSContext *ctx, void *user_data);`
- `void JS_SetContextUserData(JSContext *ctx, void *user_data, JSContextUserDataFinalizer fin);`
- `void *JS_GetContextUserData(JSContext *ctx);`

底层新增 `ctx->user_data` 字段（不与 `ctx->opaque` 冲突）。

> 若短期必须复用 `ctx->opaque`，则需要把 `CtxExt` 设计成同时包含 write_func/interrupt 所需 opaque，并统一规定 write_func/interrupt_handler 的 opaque 都从 `CtxExt` 内部取；此路线更脆弱，不建议作为默认方案。

### 3.3 生命周期与释放（非常重要）

- `ctx->opaque` 继续保留给引擎 core（输出/中断回调）。
- ridl 扩展的 `CtxExt` 通过 `JS_SetContextUserData(ctx, ext_ptr, finalizer)` 绑定。
- 在 `JS_FreeContext` 内部调用该 finalizer，释放 `CtxExt`，从而保证 RTOS 下严格无泄漏。

约束：finalizer **只做 drop/free**，不允许调用 JS API。

---

## 4) console 的 glue 生成与 stdlib 注册方式

### 4.1 stdlib 注册（JS_OBJECT_DEF）

保持 `mqjs_stdlib.c` 的模式：console 是 global property：

- `JS_PROP_CLASS_DEF("console", &js_console_obj),`

对象描述表中列出：

- `log` / `error`：函数
- `enabled`：getter-only 属性

### 4.2 glue 函数逻辑（实例从 ctx 取）

`log`：

- `ext_ptr = JS_GetContextUserData(ctx)`
- `console = (&mut *ext_ptr).console.as_mut()`
- varargs any：把 argv 全部作为 `&[JSValue]` 传给 trait
- 返回 `JS_UNDEFINED`
- 若 ext 为空/null：抛 TypeError 并返回 `JS_EXCEPTION`

`enabled getter`：

- 同样取 ext.console
- 调用 `enabled(ctx)` 返回 bool
- 构造 JS bool 返回

### 4.3 strict/默认模式

- `console.log/error` 的参数是 any varargs，不触发 strict 转换限制（与既定决策一致）
- `enabled` 返回 bool，构造 JS bool，无转换风险

---

## 5) 多 JSContext 隔离保证

隔离来源：

- 每个 `Context::new` 都创建自己的 `CtxExt { console: Box<dyn ConsoleSingleton> }`
- glue 每次通过 `ctx` 找 ext

不使用任何全局可变状态，不会跨 task 串台。

> 可选：如果 output 设备是全局的（串口），那是 IO 层共享，不算“console 状态串台”；但 enabled/配置/前缀等必须在 ext 内。

---

## 6) 需要改动的代码面（实施清单）

### A) ridl-tool

- parser：新增 `singleton <name> { ... }`（methods + readonly property）
- validator：varargs/strict/命名冲突检查
- generator：生成 `ConsoleSingleton` trait +（可选）生成 stdlib glue 声明骨架

### B) mquickjs（C 侧）

- **不复用 `ctx->opaque`**（它已被 mquickjs 内部用作输出/中断回调的 opaque）。
- 增加“扩展专用”的 ctx user_data API（h + c）：
  - `typedef void (*JSContextUserDataFinalizer)(JSContext *ctx, void *user_data);`
  - `void JS_SetContextUserData(JSContext *ctx, void *user_data, JSContextUserDataFinalizer fin);`
  - `void *JS_GetContextUserData(JSContext *ctx);`
- 并在 `JSContext` 内部结构中新增：
  - `user_data`
  - `user_data_finalizer`
- 并在 `JS_FreeContext` 内部调用该 finalizer。
  - finalizer **仅允许释放内存（drop/free）**，不允许调用任何 JS API。

> 这是隔离/无泄漏方案的关键基础设施。

### C) mquickjs-rs

- mquickjs-rs **保持为不感知 ridl modules 的框架库**：不在 `mquickjs_rs::Context::new` 中调用任何 ridl 初始化。
- 仍然通过 bindgen 暴露底层 C API（包括 `JS_SetContextUserData/JS_GetContextUserData`）。

### C.1) 应用层（facade）：每个使用 mquickjs-rs 的项目自行聚合（**聚合生成层落地**）

#### C.1.1 谁创建 `ConsoleSingleton` 实例？

- **由最终应用/集成层创建，并绑定到每个 JSContext 的 user_data**。
- 具体实现载体：由 ridl-tool 在构建期生成的 `ridl_context_init.rs`（位于应用 crate 的 `$OUT_DIR`）。

> 原则：mquickjs-rs 保持“框架库”定位，不内置任何 RIDL module/stdlib 的初始化调用。

#### C.1.2 单一所有权：`CtxExt` 由“聚合层”统一创建与释放

- `CtxExt` 是“所有 singleton 的 per-ctx 实例聚合容器”。
- **只有聚合层**负责：
  - 分配/构造 `CtxExt`
  - 调用 `JS_SetContextUserData(ctx, ext_ptr, finalizer)`
  - 在 `JS_FreeContext` 触发 finalizer 时释放 `CtxExt`

这样可避免“多个模块分别 set user_data / 分别释放”的责任冲突。

#### C.1.3 每个 RIDL module 生成自己的 `*_context_init`，聚合层统一调用（**A：类型擦除槽位**）

问题：singleton 的 trait（例如 `RidlConsoleSingleton`）定义在对应 ridl module 内；若 `CtxExt.console: Box<dyn RidlConsoleSingleton>`，聚合层将被迫感知该 trait，破坏隔离。

方案A：`CtxExt` 不存放具体 trait 类型，而是存放“**类型擦除的槽位**”（opaque ptr + dropper），从而做到：
- trait 类型完全隐藏在模块内部
- 聚合层只负责生命周期与 user_data 绑定/释放

##### C.1.3.1 `CtxExt` 数据结构（聚合层可见、无模块 trait 依赖）

```rust
use core::ffi::c_void;

pub struct ErasedSingletonSlot {
    ptr: *mut c_void,
    drop_fn: Option<unsafe fn(*mut c_void)>,
}

impl ErasedSingletonSlot {
    pub const fn empty() -> Self {
        Self { ptr: core::ptr::null_mut(), drop_fn: None }
    }

    pub fn is_set(&self) -> bool {
        !self.ptr.is_null()
    }

    /// Safety: slot must be empty or already dropped; drop_fn must match ptr allocation.
    pub unsafe fn set(&mut self, ptr: *mut c_void, drop_fn: unsafe fn(*mut c_void)) {
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

pub struct CtxExt {
    pub console: ErasedSingletonSlot,
    // future singletons...
}

impl CtxExt {
    pub const fn new() -> Self {
        Self { console: ErasedSingletonSlot::empty() }
    }

    /// Safety: called from JSContext finalizer; must not call any JS API.
    pub unsafe fn drop_all(&mut self) {
        self.console.drop_in_place();
        // drop future singletons...
    }
}
```

> 说明：聚合层只依赖 `c_void` 与函数指针，无任何模块内 trait 名。

##### C.1.3.2 模块侧初始化：创建 `Box<dyn Trait>` 并擦除写入槽位

每个 ridl module 生成一个导出函数（聚合层只需调用它）：

```rust
// in stdlib (generated)
use core::ffi::c_void;

pub fn ridl_module_context_init(ext: &mut CtxExt) {
    // 1) create impl as trait object (trait is module-private detail)
    let b: Box<dyn RidlConsoleSingleton> = Box::new(DefaultConsole::new());

    // 2) erase into raw ptr
    let p: *mut (dyn RidlConsoleSingleton) = Box::into_raw(b);

    // 3) provide dropper that knows the real type
    unsafe fn drop_console(p: *mut c_void) {
        let p = p as *mut (dyn RidlConsoleSingleton);
        drop(Box::from_raw(p));
    }

    // 4) store into slot
    unsafe {
        ext.console.set(p as *mut c_void, drop_console);
    }
}
```

> 模块内部可以自由使用 `RidlConsoleSingleton`，但它不出现在 `CtxExt` 的字段类型中，因此聚合层不需要感知。

##### C.1.3.3 glue 访问：从槽位恢复 trait object 并调用

对应 singleton 的 glue 仍生成在该模块内，因此它也能引用该 trait：

```rust
// in stdlib glue (generated)
use core::ffi::c_void;

fn get_console(ext: &mut CtxExt) -> &mut dyn RidlConsoleSingleton {
    if !ext.console.is_set() {
        // throw TypeError in caller
        unreachable!();
    }
    let p = ext.console.ptr as *mut (dyn RidlConsoleSingleton);
    unsafe { &mut *p }
}
```

> 这里 `ErasedSingletonSlot.ptr` 需要提供受控访问（例如 `fn ptr(&self)->*mut c_void`），避免外部随意解引用；此处仅示意。

##### C.1.3.4 聚合层 `ridl_context_init.rs`：只负责调用各模块 init + 绑定 user_data

```rust
pub unsafe fn ridl_context_init(ctx: *mut JSContext) {
    let mut ext = CtxExt::new();

    // call each module’s initializer
    stdlib::ridl_module_context_init(&mut ext);
    // other modules...

    // one-time bind to ctx user_data + finalizer
    JS_SetContextUserData(ctx, Box::into_raw(Box::new(ext)).cast(), Some(finalizer));
}

unsafe extern "C" fn finalizer(_ctx: *mut JSContext, user_data: *mut c_void) {
    // MUST NOT call JS API
    let mut b = Box::from_raw(user_data as *mut CtxExt);
    b.drop_all();
    // Box drops here
}
```

> 约束：模块初始化函数只做“构造/装配 ext 槽位”，**不调用 JS API**，不触碰 user_data 绑定。

#### C.1.4 聚合接口如何导入：使用方提供 facade `Context`

- 由“最终应用/集成层”crate（本仓库根 crate：`mquickjs_demo`）提供稳定入口 `mquickjs_demo::Context`。
- 该 wrapper 只做两件事：
  1) 调用 `mquickjs_rs::Context::new` 创建 JSContext
  2) 立即调用构建期生成的 `ridl_context_init(ctx_raw)` 完成“无感初始化”（创建 `CtxExt` 并 `JS_SetContextUserData(ctx, ext_ptr, fin)`）

新增/删除 ridl modules 仅影响生成的 `ridl_context_init.rs` 的聚合调用列表，不会冲击 mquickjs-rs 的 API。

> 约定：`ridl_context_init` 是 context 级初始化；与“进程级符号 keep-alive/集中注册”分离，二者不可混用。

### D) stdlib 模块

- console RIDL 改为新 singleton body 语法
- 提供默认 `ConsoleSingleton` 实现（基于你们现有打印逻辑）

### E) tests/smoke

- 现有 console log varargs smoke 保持
- 新增 `console.enabled` getter smoke
- 新增多 ctx 隔离 smoke（如果测试框架支持在一个进程里创建两个 Context 并分别 eval）

---

## 7) 风险点与对策

- 不复用 `ctx->opaque`：避免破坏引擎 core 的输出/中断回调行为。
- user_data finalizer 只做内存释放：禁止调用 JS API，避免 FreeContext 阶段的未定义行为。
- RTOS 下并发：若多个 task 同时用同一个 ctx，需要外部同步；本方案只保证“不同 ctx 互不影响”。

---

## 待确认点

1) mquickjs 的 user_data finalizer 是否确定只做 drop/free（不允许 JS API）？（✅ 已确认）
2) `Context::new` 是否需要允许注入自定义 console 实现？（例如 builder；否则仅默认实现）
3) `ConsoleSingleton` 方法签名使用 `&mut self` 还是 `&self`？（建议 `&mut self`）
