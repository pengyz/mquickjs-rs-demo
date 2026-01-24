use std::cell::UnsafeCell;
use std::marker::PhantomData;

use crate::handles::local::Local;
use crate::handles::scope::{ContextId, Scope};
use crate::mquickjs_ffi;

pub struct Global<T = crate::handles::local::Value> {
    ctx: *mut mquickjs_ffi::JSContext,
    ctx_id: ContextId,
    inner: std::sync::Arc<crate::context::ContextInner>,
    /// Safety: this cell holds a JSGCRef linked into ctx list via JS_AddGCRef.
    gc_ref: std::pin::Pin<Box<UnsafeCell<mquickjs_ffi::JSGCRef>>>,
    _t: PhantomData<T>,
}

impl<T> Global<T> {
    pub fn new<'ctx>(scope: &Scope<'ctx>, v: Local<'ctx, T>) -> Self {
        assert_eq!(v.ctx_id(), scope.context_id(), "cross-context Global::new");

        let gc_ref = Box::pin(UnsafeCell::new(mquickjs_ffi::JSGCRef {
            val: mquickjs_ffi::JS_UNDEFINED,
            prev: std::ptr::null_mut(),
        }));

        unsafe {
            let slot = mquickjs_ffi::JS_AddGCRef(scope.ctx_raw(), gc_ref.as_ref().get_ref().get());
            *slot = v.as_raw();
        }

        Self {
            ctx: scope.ctx_raw(),
            ctx_id: scope.context_id(),
            inner: scope.h.inner.clone(),
            gc_ref,
            _t: PhantomData,
        }
    }

    pub fn reset<'ctx>(&mut self, scope: &Scope<'ctx>, v: Local<'ctx, T>) {
        assert_eq!(
            self.ctx_id,
            scope.context_id(),
            "cross-context Global::reset(scope)"
        );
        assert_eq!(
            v.ctx_id(),
            scope.context_id(),
            "cross-context Global::reset(value)"
        );

        unsafe {
            let p = self.gc_ref.as_ref().get_ref().get();
            (*p).val = v.as_raw();
        }
    }

    pub fn reset_empty(&mut self) {
        unsafe {
            let p = self.gc_ref.as_ref().get_ref().get();
            (*p).val = mquickjs_ffi::JS_UNDEFINED;
        }
    }

    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        unsafe { (*self.gc_ref.as_ref().get_ref().get()).val }
    }

    pub fn ctx_id(&self) -> ContextId {
        self.ctx_id
    }
}

impl<T> Drop for Global<T> {
    fn drop(&mut self) {
        if !self.inner.alive.load(std::sync::atomic::Ordering::Acquire) {
            panic!("Global must be dropped before Context drop");
        }

        unsafe {
            mquickjs_ffi::JS_DeleteGCRef(self.ctx, self.gc_ref.as_ref().get_ref().get());
        }
    }
}
