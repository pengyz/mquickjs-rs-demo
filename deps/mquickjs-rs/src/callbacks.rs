//! 同步 many-shot 回调桥 —— `CallbackRegistry`（设计切片 1）。
//!
//! 设计文档：`docs/superpowers/specs/2026-10-10-callback-bridge-design.md`
//! （§2 语义模型 / §3 切片 1）。
//!
//! # 能力边界
//!
//! - **同步**：C 事件点直接跑 JS（mquickjs 单线程，无竞态）；
//!   fire-and-forget 的异步完成通知是另一个正交能力（`async_bridge`，
//!   本模块不改它）。
//! - **many-shot**：同一 [`CallbackHandle`] 可 `invoke` 任意多次。
//! - **异常受控**：`invoke` 把 JS 异常转为 `Err(JsException(msg))`，且
//!   **返回 `Err` 时 ctx 必无残留 pending exception** —— 包括异常对象在
//!   stringify 期间再抛的嵌套异常（`JS_ToCString` → `JS_ToString` →
//!   `JS_ToPrimitive` 会调用**用户定义的** toString/valueOf，见
//!   `mquickjs.c:4285-4310`；其再抛的新异常由 invoke 清扫）。异常绝不
//!   长跳穿透 C 栈（`JS_GetException` 取走即清除，`mquickjs.c:2139` 置
//!   `current_exception = JS_UNINITIALIZED`）。
//! - **v1 无返回值**：设计非目标（callback 语法本就无返回类型）；
//!   `invoke` 忽略 `JS_Call` 的返回值。
//!
//! # GC 安全：为什么槽位必须用 `JSGCRef`
//!
//! mquickjs 的每一次 `JS_GC` 都会压缩堆（`JS_GC2` → `gc_compact_heap`，
//! memmove 前移存活对象）。引擎只重定位它自己知道的指针，`JSGCRef` 链是
//! 唯一在 mark（`mquickjs.c:12319-12323`）与重定位
//! （`mquickjs.c:12592-12596`）**两个阶段**都被扫描的持有机制 —— 完整论证
//! 见 `crate::roots` 模块文档与 `tests/gc_compaction.rs` 的差分复现。
//! 自建表存裸 `JSValue` 在首次压缩后即悬垂，因此本模块完全复用
//! `RootsRegistry` 的已验证机制：每个回调函数存于一个 `Box<JSGCRef>` 槽位。
//!
//! # 装配与 API 分层
//!
//! - [`CallbackSlots`]（`pub(crate)`）：机械层，逐方法对齐
//!   [`crate::roots::RootsRegistry`]（`insert` / `get` / `remove`，ctx 由
//!   调用方传入）。由 `ContextInner` 持有；Context 销毁时随其静默 drop
//!   （只释放 `Box`，不触碰引擎 API —— 此时引擎的 gc_ref 链已随 context
//!   内存消失）。
//! - [`CallbackRegistry`]（pub）：façade，经 `Context::callbacks()` /
//!   `ContextToken::callbacks()` 获得，提供设计文档 §2 的 API 面
//!   （`register` / `invoke` / `unregister`）。
//!
//! # 为什么方法是 `&self` 而非 `&mut self`
//!
//! registry 由 `Arc<ContextInner>` 共享（对齐 `RootsRegistry` 装配方式），
//! 切片 2 的 C trampoline 经 `ContextToken::from_js_ctx` 只能拿到共享引用，
//! `&mut self` 无法满足该路径。槽位表用 `Mutex`（std）/ `RefCell`
//! （no-std）做内部可变；**锁绝不跨 `JS_Call` 持有**（读出槽位值后立即
//! 释放），回调体内再入（一个回调触发另一个回调 / 注销自己）不会自锁，
//! 也不会损坏表 —— 被调函数值在 `invoke` 前已压入引擎值栈，由引擎保活
//! 并重定位。
//!
//! # 线程约束
//!
//! [`CallbackRegistry`] 含裸 ctx 指针 ⇒ `!Send + !Sync`，与 JS 单线程
//! 模型一致；所有方法必须在 JS 线程调用（与其它 `Context` 操作同线程）。

#[cfg(feature = "no-std")]
use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};
use core::cell::UnsafeCell;
use core::ffi::CStr;

#[cfg(not(feature = "no-std"))]
use std::sync::Mutex;
#[cfg(feature = "no-std")]
use core::cell::RefCell;

use crate::handles::local::{Function, Local};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi::{self, JSContext, JSGCRef, JSValue};

// JS 本身单线程；与 `RootsRegistry` 一致：std 模式用 `Mutex`
// （容忍跨线程共享注册表本身），no-std 模式用 `RefCell`。
#[cfg(not(feature = "no-std"))]
type Slots = Mutex<Vec<Option<Box<JSGCRef>>>>;
#[cfg(feature = "no-std")]
type Slots = RefCell<Vec<Option<Box<JSGCRef>>>>;

#[cfg(not(feature = "no-std"))]
macro_rules! lock_slots {
    ($this:ident) => {
        $this.slots.lock().expect("CallbackSlots poisoned")
    };
}
#[cfg(feature = "no-std")]
macro_rules! lock_slots {
    ($this:ident) => {
        $this.slots.borrow_mut()
    };
}

/// 回调句柄：[`CallbackSlots`] 槽位表的索引。
///
/// `Copy + Eq + Hash`（可存可传 —— LVGL user_data 形状）；内部字段私有，
/// 伪造的句柄在 `invoke` 时得到 `InvalidHandle`，不会触达引擎。
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CallbackHandle(u32);

impl CallbackHandle {
    /// 从原始 `u32` 构造句柄（**纯构造器，无引擎交互**）。
    ///
    /// 用途：生成的 C trampoline（`mqjs_cb_<name>_invoke`，切片 2 codegen）
    /// 的 ABI 参数是 `uint32_t handle`——句柄经 Rust impl 的 [`Self::raw`]
    /// 存入 C user_data，再在 trampoline 里还原后调用 [`CallbackRegistry::invoke`]。
    /// 伪造的值不会触达引擎：`invoke` 时得到 `InvalidHandle`。
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 取原始 `u32`（**纯访问器**）——Rust impl 把句柄存入/传给 C 侧
    /// user_data 时使用（LVGL `lv_obj_add_event_cb(user_data)` 形状）。
    pub fn raw(self) -> u32 {
        self.0
    }
}

/// `invoke` 的错误。
///
/// 除 `JsException` 携带 JS 异常消息外，其余变体**均不产生任何引擎副作用**
/// （无 pending exception 残留）。
#[derive(Debug, Clone)]
pub enum CallbackError {
    /// 句柄未注册或已 `unregister`。
    InvalidHandle,
    /// 回调执行抛出 JS 异常。异常消息已捕获，pending exception 已清除，
    /// ctx 可继续正常工作。
    JsException(String),
    /// JS 值栈空间不足（`JS_StackCheck` 失败）。
    StackOverflow,
    /// Context 已销毁（经存活的 `ContextToken` 访问）。
    ContextDropped,
}

impl core::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CallbackError::InvalidHandle => write!(f, "invalid callback handle"),
            CallbackError::JsException(msg) => write!(f, "callback threw: {msg}"),
            CallbackError::StackOverflow => write!(f, "js stack overflow for callback"),
            CallbackError::ContextDropped => write!(f, "context dropped"),
        }
    }
}

impl core::error::Error for CallbackError {}

/// 回调槽位表（机械层）—— 逐方法对齐 [`crate::roots::RootsRegistry`]。
///
/// 每个槽位持有一个 `Box<JSGCRef>`；`Box` 保证 `JSGCRef` **地址稳定**
/// （引擎把该节点链入 `ctx->last_gc_ref`，若节点自身被移动，引擎链表即
/// 损坏 —— 见 `RootsRegistry::slots` 的注释）。空槽复用。
pub(crate) struct CallbackSlots {
    /// slot → `Box<JSGCRef>`；`None` 为已释放槽位，可被复用。
    slots: Slots,
    // 使类型在 API 边界默认 !Send/!Sync（对齐 RootsRegistry；
    // JSContext 的线程模型不作保证）。
    _no_send: UnsafeCell<()>,
}

impl CallbackSlots {
    pub(crate) fn new() -> Self {
        Self {
            slots: <Slots as Default>::default(),
            _no_send: UnsafeCell::new(()),
        }
    }

    /// 注册一个回调函数值，返回其句柄。
    ///
    /// # Safety
    ///
    /// - 必须在 JS 线程调用
    /// - `ctx` 必须有效且未销毁
    pub(crate) unsafe fn insert(&self, ctx: *mut JSContext, fn_raw: JSValue) -> CallbackHandle {
        debug_assert!(!ctx.is_null(), "CallbackSlots::insert with null ctx");

        let mut r = Box::new(JSGCRef {
            val: mquickjs_ffi::JS_UNDEFINED,
            prev: core::ptr::null_mut(),
        });

        // JS_AddGCRef 把 r 链入 ctx->last_gc_ref 并返回 &mut r.val。
        // 此后每次 GC 压缩，引擎都会直接更新 *slot 指向的位置（重定位）。
        let slot = unsafe { mquickjs_ffi::JS_AddGCRef(ctx, &mut *r) };
        if !slot.is_null() {
            unsafe { *slot = fn_raw };
        }

        // 空槽复用（unregister 产生的洞优先填补），与 RootsRegistry 一致。
        let mut g = lock_slots!(self);
        for (i, s) in g.iter_mut().enumerate() {
            if s.is_none() {
                *s = Some(r);
                return CallbackHandle(i as u32);
            }
        }
        let id = g.len();
        g.push(Some(r));
        CallbackHandle(id as u32)
    }

    /// 读取槽位**当前**持有的函数值。
    ///
    /// 读的是引擎维护的 `JSGCRef.val` —— 即 GC 压缩重定位后的最新地址。
    /// 锁在此调用内取得并释放（调用方不得跨 `JS_Call` 持有返回值对应的锁
    /// —— 本身也不持有，返回的是 `JSValue` 位拷贝）。
    pub(crate) fn get(&self, handle: CallbackHandle) -> Option<JSValue> {
        let g = lock_slots!(self);
        g.get(handle.0 as usize)?.as_ref().map(|r| r.val)
    }

    /// 注销槽位并释放其 `JSGCRef`（函数自此可被 GC）。
    ///
    /// 返回该句柄原本是否持有槽位。
    ///
    /// # Safety
    ///
    /// - 必须在 JS 线程调用，且 `ctx` 仍然有效（`inner.alive` 为真）
    pub(crate) unsafe fn remove(&self, ctx: *mut JSContext, handle: CallbackHandle) -> bool {
        let taken = {
            let mut g = lock_slots!(self);
            match g.get_mut(handle.0 as usize) {
                Some(s) => s.take(),
                None => None,
            }
        };

        match taken {
            Some(mut r) => {
                if !ctx.is_null() {
                    // 从引擎链表摘除；随后 Box 释放，其堆地址不再被引用。
                    unsafe { mquickjs_ffi::JS_DeleteGCRef(ctx, &mut *r) };
                }
                drop(r);
                true
            }
            None => false,
        }
    }
}

/// 同步 many-shot 回调桥的公共 API 面（设计文档 §2）。
///
/// 经 `Context::callbacks()` / `ContextToken::callbacks()` 获得；
/// 生命周期绑定到所属 Context / ContextToken。含裸 ctx 指针 ⇒
/// `!Send + !Sync`。
///
/// # 与 async bridge 的边界
///
/// 本类型是**同步**触发（C 事件点直接跑 JS）；跨线程入队 / 完成通知属
/// `async_bridge`（正交能力）。
pub struct CallbackRegistry<'a> {
    /// 所属 context 的原始指针。有效性由 `inner.alive` 护栏保证
    /// （对齐 `Root` 持有 `ctx` 的方式）。
    ctx: *mut JSContext,
    inner: &'a crate::context::ContextInner,
}

impl<'a> CallbackRegistry<'a> {
    pub(crate) fn new(ctx: *mut JSContext, inner: &'a crate::context::ContextInner) -> Self {
        Self { ctx, inner }
    }

    /// 当前 context 是否仍存活。
    fn alive(&self) -> bool {
        self.inner
            .alive
            .load(core::sync::atomic::Ordering::Acquire)
    }

    /// 注册一个 JS 回调函数，返回可在 C/Rust 侧传递的句柄。
    ///
    /// 函数值进入 `JSGCRef` 槽位（GC 安全：引擎自动标记 + 压缩重定位），
    /// 直至 [`Self::unregister`] 或 Context 销毁。
    ///
    /// # Panics
    ///
    /// - `scope` 与本 registry 不属于同一 context（跨 context 是硬错误，
    ///   对齐 `Root::new` 的 assert 风格）
    /// - context 已销毁（经存活 `ContextToken` 注册是编程错误；
    ///   断言在任何引擎调用**之前**执行，不会触碰悬垂指针）
    ///
    /// # 契约
    ///
    /// `Local<Function>` 类型即"是函数"的契约 —— RIDL glue（切片 2）负责
    /// `JS_IsFunction` 校验后才构造它；这里仅以 `debug_assert` 早暴露。
    pub fn register(&self, scope: &Scope<'_>, fn_local: Local<'_, Function>) -> CallbackHandle {
        assert_eq!(
            scope.ctx_raw(),
            self.ctx,
            "cross-context callback register"
        );
        assert!(
            self.alive(),
            "callback registry: context dropped",
        );

        let raw = fn_local.as_raw();
        debug_assert!(
            unsafe { mquickjs_ffi::JS_IsFunction(self.ctx, raw) } != 0,
            "callback register: value is not a function",
        );

        // Safety: alive 断言通过 ⇒ ctx 有效；`&Scope` 证明当前处于 JS 线程。
        unsafe { self.inner.callback_slots.insert(self.ctx, raw) }
    }

    /// 同步调用句柄持有的 JS 函数（many-shot：可调用任意多次）。
    ///
    /// `this` 绑定为 `undefined`（事件语义，与 `drain_completions` 一致）。
    ///
    /// # 异常策略（设计文档 §2）
    ///
    /// JS 异常 → `Err(JsException(msg))`；`JS_GetException` 已取走并清除
    /// pending exception，异常不穿透 C 栈，ctx 可继续工作。
    ///
    /// # 参数的 GC 约束
    ///
    /// `args` 中的**立即值**（int / bool / null / undefined）不受 GC 影响，
    /// 可任意传。堆对象值（string / object / function）只有在**压入引擎
    /// 值栈之后**才由引擎保活并随压缩重定位；压栈之前（含可能触发 GC 的
    /// `JS_StackCheck`，见下）传入的裸副本依赖**调用方持有** —— 堆对象
    /// 参数应先经 GC 安全句柄（`Root` / `JSGCRef`）持有，并改用
    /// [`Self::invoke_rooted`]（本方法在 `JS_StackCheck` 之前压栈的裸副本
    /// 可能在其触发的压缩后悬垂）。调用返回后，调用方持有的裸 `JSValue`
    /// 副本一律不得再解引用。切片 2 codegen 的立即值参数（i32 / bool）天然
    /// 满足；string 与一般 f64（非短整型的 double 会在引擎堆上分配
    /// `JSFloat64`，`mquickjs.c:1009`）一律改走 [`Self::invoke_rooted`]。
    ///
    /// # 锁与再入
    ///
    /// 槽位锁在读取函数值后立即释放，**不跨 `JS_Call` 持有** —— 回调体内
    /// 再入（触发其它回调、注销自己）安全。
    pub fn invoke(&self, handle: CallbackHandle, args: &[JSValue]) -> Result<(), CallbackError> {
        let argc = args.len() as u32;
        self.invoke_impl(handle, argc, &mut |out| {
            out.extend_from_slice(args);
        })
    }

    /// [`Self::invoke`] 的 GC 安全重载：堆对象参数以 **`Root`（引擎
    /// `JSGCRef` 持有，压缩时自动重定位）** 传入。
    ///
    /// 为什么需要它（切片 2 C trampoline 的 string/f64 参数）：`invoke` 的
    /// `&[JSValue]` 是裸位拷贝，而本模块内唯一可能触发 GC/堆压缩的点是
    /// `JS_StackCheck`（`check_free_mem` 堆不足时直接 `JS_GC`，见
    /// `mquickjs.c:529`）——裸拷贝在压缩后即悬垂，再压栈就是 UAF。
    /// 本重载把参数值的读取推迟到 `JS_StackCheck` **之后**、压栈之前，
    /// 经 `Root::as_raw()` 重读引擎维护的 `JSGCRef.val`，拿到的是重定位后
    /// 的最新地址 —— 与下方读取回调函数槽位的顺序完全一致。
    ///
    /// 立即值参数也可以包进 `Root`（`JSGCRef` 对非指针值是 no-op），
    /// 因此 codegen 对"含任一堆值参数"的回调统一走本方法。
    pub fn invoke_rooted(
        &self,
        handle: CallbackHandle,
        args: &[&crate::roots::Root<crate::handles::local::Value>],
    ) -> Result<(), CallbackError> {
        let argc = args.len() as u32;
        self.invoke_impl(handle, argc, &mut |out| {
            out.extend(args.iter().map(|r| r.as_raw()));
        })
    }

    /// `invoke` / `invoke_rooted` 的公共实现。
    ///
    /// `push_args` 在 `JS_StackCheck`（本方法内唯一的 GC 点）**之后**被调用，
    /// 负责把最终参数值填进 `out` —— 这是堆对象参数压缩安全的全部依据。
    /// `argc` 是参数个数（`JS_StackCheck` 的预留大小 = `argc + 2`，含
    /// fn 与 this 两个隐式槽位，与原 `invoke` 一致）。
    fn invoke_impl(
        &self,
        handle: CallbackHandle,
        argc: u32,
        push_args: &mut dyn FnMut(&mut Vec<JSValue>),
    ) -> Result<(), CallbackError> {
        if !self.alive() {
            return Err(CallbackError::ContextDropped);
        }

        // Safety: alive ⇒ ctx 有效；本类型 !Send ⇒ 处于 JS 线程。
        //
        // JS_StackCheck **可能触发 GC**（mquickjs.c:537-538 "May trigger a
        // GC()"；check_free_mem 堆不足时直接 JS_GC，而每次 JS_GC 都压缩堆，
        // mquickjs.c:529）。因此它必须在读取槽位**之前**执行：槽位经
        // JSGCRef 持有，check 之后重读即得压缩重定位后的新地址；若先读后
        // check，读出的 fn_val 位拷贝会在压缩后悬垂（function.rs::call 无
        // 此修复优势 —— 其 this/args 不经 JSGCRef，无法重读）。
        if unsafe { mquickjs_ffi::JS_StackCheck(self.ctx, argc + 2) } != 0 {
            return Err(CallbackError::StackOverflow);
        }

        // 读出槽位值（锁已释放）。此刻到压栈之间无 JS 执行、也无 GC ——
        // 唯一可能触发 GC 的 JS_StackCheck 已在读取之前完成，故该副本截至
        // 压栈必然有效；压栈后由引擎值栈保活并重定位。
        let fn_val = self
            .inner
            .callback_slots
            .get(handle)
            .ok_or(CallbackError::InvalidHandle)?;

        // 参数在 StackCheck 之后现场产出（invoke_rooted 的压缩安全性依据）。
        let mut args: Vec<JSValue> = Vec::new();
        push_args(&mut args);

        // JS_Call 的值栈约定（与 handles/function.rs::call、
        // context.rs::drain_completions 一致）：
        // 自底向上 [args 逆序..., func, this]，call_flags = argc。
        unsafe {
            for arg in args.iter().rev() {
                mquickjs_ffi::JS_PushArg(self.ctx, *arg);
            }
            mquickjs_ffi::JS_PushArg(self.ctx, fn_val);
            mquickjs_ffi::JS_PushArg(self.ctx, mquickjs_ffi::JS_UNDEFINED);

            let result = mquickjs_ffi::JS_Call(self.ctx, args.len() as i32);

            if mquickjs_ffi::js_value_special_tag(result) == mquickjs_ffi::JS_TAG_EXCEPTION {
                // 取走并清除 pending exception（mquickjs.c:2139 置
                // current_exception = JS_UNINITIALIZED）—— 不穿透 C。
                let exception = mquickjs_ffi::JS_GetException(self.ctx);

                // 注意：JS_ToCString → JS_ToString → JS_ToPrimitive 会执行
                // **用户定义的** toString/valueOf（mquickjs.c:4285-4310），
                // 并非只走内建转换；若用户转换函数再抛，JS_ToCString 返回
                // null 且**新异常留在 ctx**。因此 null 分支必须清扫嵌套
                // 异常（见下），保证"返回 Err ⇒ ctx 无残留"的模块承诺。
                let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
                let msg_ptr = mquickjs_ffi::JS_ToCString(self.ctx, exception, &mut cstr_buf);
                let msg = if !msg_ptr.is_null() {
                    CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
                } else if mquickjs_ffi::JS_HasException(self.ctx) != 0 {
                    // 嵌套 throw：取走并清除 stringify 期间产生的新异常。
                    // （嵌套异常值取出后立即丢弃 —— 无引用计数，不持有即
                    // 无泄漏；原异常的位拷贝在用户代码执行期间可能因 GC
                    // 失效，此处不再解引用它。）
                    let _nested = mquickjs_ffi::JS_GetException(self.ctx);
                    "callback exception (also thrown while stringifying)".to_string()
                } else {
                    "Unknown callback exception".to_string()
                };
                return Err(CallbackError::JsException(msg));
            }
        }

        // v1 忽略返回值（设计非目标）。返回值未被持有 ⇒ 不可达 ⇒ 可被 GC；
        // mquickjs 无引用计数，不持有即无泄漏。
        Ok(())
    }

    /// 注销句柄并释放其 `JSGCRef` 槽位（函数自此可被 GC）。
    ///
    /// 返回该句柄原本是否持有槽位（重复注销返回 `false`）。
    /// Context 已销毁时安全地返回 `false`（对齐 `Root::drop`：引擎链表已
    /// 消失，不能再调引擎 API；槽位随 `ContextInner` drop 释放）。
    pub fn unregister(&self, handle: CallbackHandle) -> bool {
        if !self.alive() {
            return false;
        }
        // Safety: alive ⇒ ctx 有效；本类型 !Send ⇒ 处于 JS 线程。
        unsafe { self.inner.callback_slots.remove(self.ctx, handle) }
    }
}
