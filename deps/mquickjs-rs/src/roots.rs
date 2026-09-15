//! GC root 注册表 —— 基于引擎 `JSGCRef` 的压缩安全实现。
//!
//! # 为什么必须用 `JSGCRef`（而不是自建 registry + `JS_SetContextGCMark`）
//!
//! mquickjs 的 `JS_GC` → `JS_GC2` → `gc_mark_all` + **`gc_compact_heap`**，
//! 即**每一次 GC 都会压缩堆**（用 memmove 移动存活对象）。
//!
//! 引擎只重定位"它自己知道的"指针，重定位清单见 `mquickjs.c:12572-12608`：
//!
//! 1. context 内建字段（`unique_strings` … `class_proto + 2*class_count`）
//! 2. `string_pos_cache[i].str`
//! 3. JS 值栈（`ctx->sp` … `stack_top`）
//! 4. **`JSGCRef` 链**（`top_gc_ref` / `last_gc_ref`）
//! 5. `parse_state` 各字段
//!
//! 通过 `JS_SetContextGCMark` 注册的回调**只在 mark 阶段被调用**
//! （`mquickjs.c:12319`），它标记的值会被保活，但**不会被重定位**。
//!
//! 因此自建 registry 存放的 `JSValue` 在压缩后即悬垂：
//! 对象被移动到新地址，而 registry 里仍是旧地址 —— 这是静默内存损坏。
//! 复现见 `deps/mquickjs-rs/tests/gc_compaction.rs`。
//!
//! `JSGCRef` 链在**两个阶段都被扫描**，是 mquickjs 下唯一正确的跨 GC 持有机制：
//!
//! - mark：`mquickjs.c:12319-12323`（`gc_mark_root(s, ref->val)`）
//! - 重定位：`mquickjs.c:12592-12596`（`gc_thread_pointer(ctx, &ref->val)`）
//!
//! # 线程约束
//!
//! `JS_AddGCRef` / `JS_DeleteGCRef` 直接操作 `ctx->last_gc_ref` 链表，
//! **必须在 JS 线程调用**。`Root<T>` 是 `!Send`，与此约束一致。

#[cfg(feature = "no-std")]
use alloc::{boxed::Box, vec::Vec};
use core::cell::UnsafeCell;

use crate::mquickjs_ffi;

// JS 本身单线程，且 `Root` 刻意是 `!Send`，因此裸机上用 `RefCell` 即可；
// std 模式保留 `Mutex`（可跨线程共享 `RootsRegistry`，虽然实践中不需要）。
#[cfg(not(feature = "no-std"))]
use std::sync::Mutex;
#[cfg(feature = "no-std")]
use core::cell::RefCell;

#[cfg(not(feature = "no-std"))]
type Slots = Mutex<Vec<Option<Box<JSGCRef>>>>;
#[cfg(feature = "no-std")]
type Slots = RefCell<Vec<Option<Box<JSGCRef>>>>;

#[cfg(not(feature = "no-std"))]
macro_rules! lock_slots {
    ($this:ident) => {
        $this.slots.lock().expect("RootsRegistry poisoned")
    };
}
#[cfg(feature = "no-std")]
macro_rules! lock_slots {
    ($this:ident) => {
        $this.slots.borrow_mut()
    };
}
use crate::mquickjs_ffi::{JSContext, JSGCRef, JSValue};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RootId(u32);

/// 基于引擎 `JSGCRef` 的 context 级 root 注册表。
///
/// 每个 root 对应一个 `Box<JSGCRef>`；引擎把该 `JSGCRef` 链接进
/// `ctx->last_gc_ref`，并在每次压缩时直接更新 `ref->val`。
pub(crate) struct RootsRegistry {
    /// slot → `Box<JSGCRef>`
    ///
    /// `Box` 保证 `JSGCRef` 的**地址稳定**：引擎持有 `&mut *r` 的指针，
    /// 若 `JSGCRef` 自身被移动，引擎的链表即损坏。
    slots: Slots,

    // 使类型在 API 边界默认 !Send/!Sync（registry 本身在 Mutex 后，
    // 但 JSContext 的线程模型不作保证）。
    _no_send: UnsafeCell<()>,
}

impl RootsRegistry {
    pub(crate) fn new() -> Self {
        Self {
            slots: <Slots as Default>::default(),
            _no_send: UnsafeCell::new(()),
        }
    }

    /// 注册一个**压缩安全**的 GC root。
    ///
    /// # Safety
    ///
    /// - 必须在 JS 线程调用
    /// - `ctx` 必须有效且未销毁
    pub(crate) unsafe fn insert(&self, ctx: *mut JSContext, v: JSValue) -> RootId {
        debug_assert!(!ctx.is_null(), "RootsRegistry::insert with null ctx");

        let mut r = Box::new(JSGCRef {
            val: mquickjs_ffi::JS_UNDEFINED,
            prev: core::ptr::null_mut(),
        });

        // JS_AddGCRef 把 r 链接进 ctx->last_gc_ref，并返回 &mut r.val。
        // 此后每次 GC 压缩，引擎都会直接更新 *slot 指向的位置。
        let slot = unsafe { mquickjs_ffi::JS_AddGCRef(ctx, &mut *r) };
        if !slot.is_null() {
            unsafe { *slot = v };
        }

        let mut g = lock_slots!(self);
        for (i, s) in g.iter_mut().enumerate() {
            if s.is_none() {
                *s = Some(r);
                return RootId(i as u32);
            }
        }
        let id = g.len();
        g.push(Some(r));
        RootId(id as u32)
    }

    /// 读取 root **当前**的值。
    ///
    /// 由于读的是引擎维护的 `JSGCRef.val`，这里拿到的是压缩重定位后的最新地址，
    /// 而不是注册时的旧副本。
    pub(crate) fn get(&self, id: RootId) -> Option<JSValue> {
        let g = lock_slots!(self);
        g.get(id.0 as usize)?.as_ref().map(|r| r.val)
    }

    /// 注销 root 并释放其 `JSGCRef`。
    ///
    /// # Safety
    ///
    /// - 必须在 JS 线程调用，且 `ctx` 仍然有效
    pub(crate) unsafe fn remove(&self, ctx: *mut JSContext, id: RootId) {
        let taken = {
            let mut g = lock_slots!(self);
            match g.get_mut(id.0 as usize) {
                Some(s) => s.take(),
                None => None,
            }
        };

        if let Some(mut r) = taken {
            if !ctx.is_null() {
                // 从引擎链表中摘除；随后 Box 释放，其堆地址不再被引用。
                unsafe { mquickjs_ffi::JS_DeleteGCRef(ctx, &mut *r) };
            }
            drop(r);
        }
    }
}

/// 绑定到特定 JS context 的持久 root。
///
/// 这是跨"宿主生命周期"（例如异步任务）保活值的首选方式，
/// 且**压缩安全**：值由引擎的 `JSGCRef` 维护，GC 移动对象后自动更新。
pub struct Root<T = crate::handles::local::Value> {
    ctx_id: crate::handles::scope::ContextId,
    inner: alloc::sync::Arc<crate::context::ContextInner>,
    id: RootId,
    /// 用于 Drop 时调 `JS_DeleteGCRef`。有效性由 `inner.alive` 护栏保证。
    ctx: *mut JSContext,
    _t: core::marker::PhantomData<T>,
    // Root 不得跨线程发送：JS 线程模型不作保证。
    _no_send: core::marker::PhantomData<UnsafeCell<()>>,
}

impl<T> Root<T> {
    /// 返回 root 当前持有的 `JSValue`。
    ///
    /// **这是 GC 重定位后的最新地址**，而非注册时的副本 —— 每次调用都从
    /// 引擎维护的 `JSGCRef.val` 重新读取。
    pub fn as_raw(&self) -> JSValue {
        self.inner
            .roots
            .get(self.id)
            .unwrap_or(mquickjs_ffi::JS_UNDEFINED)
    }

    pub fn ctx_id(&self) -> crate::handles::scope::ContextId {
        self.ctx_id
    }

    /// 将 Root 转换为 Local
    ///
    /// # Safety
    /// - 必须在同一个 Context 中调用
    /// - 调用者必须确保值在使用期间有效
    pub fn to_local<'ctx>(
        &self,
        scope: &crate::handles::scope::Scope<'ctx>,
    ) -> crate::handles::local::Local<'ctx, T> {
        assert_eq!(
            scope.context_id(),
            self.ctx_id,
            "cross-context Root::to_local",
        );
        crate::handles::local::Local::from_raw_for_same_ctx(self.as_raw())
            .with_ctx_id(self.ctx_id)
    }
}

impl<'ctx, T> Root<T> {
    pub fn new(
        scope: &'ctx crate::handles::scope::Scope<'ctx>,
        v: crate::handles::local::Local<'ctx, T>,
    ) -> Self {
        assert_eq!(v.ctx_id(), scope.context_id(), "cross-context Root::new");

        let raw = v.as_raw();
        let inner = scope.h.inner.clone();
        let ctx = scope.ctx_raw();

        // Safety: scope 保证当前在 JS 线程且 ctx 有效。
        let id = unsafe { inner.roots.insert(ctx, raw) };

        Self {
            ctx_id: scope.context_id(),
            inner,
            id,
            ctx,
            _t: core::marker::PhantomData,
            _no_send: core::marker::PhantomData,
        }
    }
}

impl<T> Drop for Root<T> {
    fn drop(&mut self) {
        // 若 Context 已销毁，其 gc_ref 链表随之消失，不能再调用任何 JS API
        // （JS_DeleteGCRef 在找不到节点时会 abort）。
        if !self
            .inner
            .alive
            .load(core::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        // Safety: Context 仍存活 ⇒ ctx 有效；Root 为 !Send ⇒ 处于 JS 线程。
        unsafe { self.inner.roots.remove(self.ctx, self.id) };
    }
}