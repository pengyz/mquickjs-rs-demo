use std::cell::Cell;
use std::marker::PhantomData;

use crate::handles::handle::Handle;
use crate::handles::local::Local;
use crate::handles::scope::{ContextId, Scope};
use crate::mquickjs_ffi;

/// V8-style handle scope.
///
/// - `Local<'ctx, T>` is a view bound to a `Scope` (context boundary), not a GC root.
/// - `Handle<'hs, 'ctx, T>` is a GC-rooted handle whose lifetime is tied to this scope.
/// - Internally it links `JSGCRef` nodes into `JSContext::top_gc_ref` via `JS_PushGCRef`.
pub struct HandleScope<'ctx> {
    scope: &'ctx Scope<'ctx>,
    ctx_id: ContextId,
    head: Cell<*mut mquickjs_ffi::JSGCRef>,
    _m: PhantomData<&'ctx Scope<'ctx>>,
}

impl<'ctx> HandleScope<'ctx> {
    pub fn new(scope: &'ctx Scope<'ctx>) -> Self {
        Self {
            scope,
            ctx_id: scope.context_id(),
            head: Cell::new(std::ptr::null_mut()),
            _m: PhantomData,
        }
    }

    pub fn scope(&self) -> &'ctx Scope<'ctx> {
        self.scope
    }

    pub fn context_id(&self) -> ContextId {
        self.ctx_id
    }

    pub fn handle<'hs, T>(&'hs mut self, v: Local<'ctx, T>) -> Handle<'hs, 'ctx, T> {
        assert_eq!(v.ctx_id(), self.ctx_id, "cross-context HandleScope::handle");
        let raw = v.as_raw();
        self.push_gc_ref(raw);
        Handle::from_raw(raw, self.ctx_id)
    }

    pub fn escapable<'hs, T>(
        &'hs mut self,
        f: impl for<'inner> FnOnce(EscapableHandleScope<'inner, 'ctx>) -> Escaped<'inner, 'ctx, T>,
    ) -> Handle<'hs, 'ctx, T> {
        let escaped = f(EscapableHandleScope::new(self.scope, self.ctx_id));
        self.push_gc_ref(escaped.raw);
        Handle::from_raw(escaped.raw, self.ctx_id)
    }

    pub(crate) fn push_gc_ref(&self, raw: mquickjs_ffi::JSValue) {
        let gc_ref = Box::into_raw(Box::new(mquickjs_ffi::JSGCRef {
            val: mquickjs_ffi::JS_UNDEFINED,
            prev: std::ptr::null_mut(),
        }));

        unsafe {
            let slot = mquickjs_ffi::JS_PushGCRef(self.scope.ctx_raw(), gc_ref);
            *slot = raw;

            (*gc_ref).prev = self.head.get();
        }

        self.head.set(gc_ref);
    }
}

impl Drop for HandleScope<'_> {
    fn drop(&mut self) {
        unsafe {
            let mut p = self.head.get();
            while !p.is_null() {
                let _ = mquickjs_ffi::JS_PopGCRef(self.scope.ctx_raw(), p);
                let prev = (*p).prev;
                drop(Box::from_raw(p));
                p = prev;
            }
        }
    }
}

pub struct EscapableHandleScope<'inner, 'ctx> {
    scope: &'inner Scope<'ctx>,
    ctx_id: ContextId,
    head: Cell<*mut mquickjs_ffi::JSGCRef>,
    _m: PhantomData<&'inner mut ()>,
}

pub struct Escaped<'inner, 'ctx, T> {
    raw: mquickjs_ffi::JSValue,
    _m: PhantomData<(&'inner mut (), &'ctx (), T)>,
}

impl<'inner, 'ctx> EscapableHandleScope<'inner, 'ctx> {
    fn new(scope: &'inner Scope<'ctx>, ctx_id: ContextId) -> Self {
        Self {
            scope,
            ctx_id,
            head: Cell::new(std::ptr::null_mut()),
            _m: PhantomData,
        }
    }

    pub fn handle<T>(&mut self, v: Local<'ctx, T>) -> Handle<'inner, 'ctx, T> {
        assert_eq!(
            v.ctx_id(),
            self.ctx_id,
            "cross-context EscapableHandleScope::handle",
        );
        let raw = v.as_raw();
        self.push_gc_ref(raw);
        Handle::from_raw(raw, self.ctx_id)
    }

    pub fn escape<T>(self, v: Handle<'inner, 'ctx, T>) -> Escaped<'inner, 'ctx, T> {
        assert_eq!(
            v.ctx_id(),
            self.ctx_id,
            "cross-context EscapableHandleScope::escape",
        );
        Escaped {
            raw: v.as_raw(),
            _m: PhantomData,
        }
    }

    fn push_gc_ref(&self, raw: mquickjs_ffi::JSValue) {
        let gc_ref = Box::into_raw(Box::new(mquickjs_ffi::JSGCRef {
            val: mquickjs_ffi::JS_UNDEFINED,
            prev: std::ptr::null_mut(),
        }));

        unsafe {
            let slot = mquickjs_ffi::JS_PushGCRef(self.scope.ctx_raw(), gc_ref);
            *slot = raw;

            (*gc_ref).prev = self.head.get();
        }

        self.head.set(gc_ref);
    }

    fn pop_all(&mut self) {
        unsafe {
            let mut p = self.head.get();
            while !p.is_null() {
                let _ = mquickjs_ffi::JS_PopGCRef(self.scope.ctx_raw(), p);
                let prev = (*p).prev;
                drop(Box::from_raw(p));
                p = prev;
            }
        }
    }
}

impl Drop for EscapableHandleScope<'_, '_> {
    fn drop(&mut self) {
        self.pop_all();
    }
}
