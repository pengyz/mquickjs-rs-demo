use std::cell::UnsafeCell;
use std::sync::Mutex;

use crate::mquickjs_ffi;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RootId(u32);

/// A context-level root registry.
///
/// - Values in this registry are treated as GC roots via JS_SetContextGCMark.
/// - This is intended for host-owned lifetimes (e.g. async tasks) where values are
///   not naturally reachable from the JS heap.
///
/// Safety model:
/// - We do not expose this type publicly; users interact via `Root<T>`.
/// - We store raw JSValue without calling JS_FreeValue/JS_DupValue (engine uses tracing GC).
/// - Removal is explicit (Drop) and prevents further marking.
pub(crate) struct RootsRegistry {
    inner: Mutex<Vec<Option<mquickjs_ffi::JSValue>>>,

    // Marker to make this type !Send/!Sync by default at the API boundary.
    // The registry itself is behind a Mutex, but Context/JSContext thread model is
    // not guaranteed. Root<T> will carry a !Send marker as well.
    _no_send: UnsafeCell<()>,
}

impl RootsRegistry {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            _no_send: UnsafeCell::new(()),
        }
    }

    pub(crate) fn insert(&self, v: mquickjs_ffi::JSValue) -> RootId {
        let mut g = self.inner.lock().expect("RootsRegistry poisoned");
        for (i, slot) in g.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(v);
                return RootId(i as u32);
            }
        }
        let id = g.len();
        g.push(Some(v));
        RootId(id as u32)
    }

    pub(crate) fn remove(&self, id: RootId) {
        let mut g = self.inner.lock().expect("RootsRegistry poisoned");
        let Some(slot) = g.get_mut(id.0 as usize) else {
            return;
        };
        *slot = None;
    }

    pub(crate) fn gc_mark(&self, mf: *const mquickjs_ffi::JSMarkFunc) {
        // Safety: mf is provided by engine during GC; only call mark_value.
        let Some(mark_value) = (unsafe { (*mf).mark_value }) else {
            return;
        };

        let g = self.inner.lock().expect("RootsRegistry poisoned");
        for v in g.iter().flatten().copied() {
            unsafe { mark_value(mf, v) };
        }
    }
}

/// A persistent root tied to a specific JS context.
///
/// This is the preferred way to keep values alive across host-owned lifetimes
/// (e.g. async tasks) without exposing GC mark mechanics.
pub struct Root<T = crate::handles::local::Value> {
    ctx_id: crate::handles::scope::ContextId,
    inner: std::sync::Arc<crate::context::ContextInner>,
    id: RootId,
    raw: mquickjs_ffi::JSValue,
    _t: std::marker::PhantomData<T>,
    // Root must not be sent across threads unless we can guarantee JSContext thread-safety.
    _no_send: std::marker::PhantomData<UnsafeCell<()>>,
}

impl<T> Root<T> {
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }

    pub fn ctx_id(&self) -> crate::handles::scope::ContextId {
        self.ctx_id
    }
}

impl<'ctx, T> Root<T> {
    pub fn new(scope: &'ctx crate::handles::scope::Scope<'ctx>, v: crate::handles::local::Local<'ctx, T>) -> Self {
        assert_eq!(v.ctx_id(), scope.context_id(), "cross-context Root::new");
        let raw = v.as_raw();
        let inner = scope.h.inner.clone();
        let id = inner.roots.insert(raw);
        Self {
            ctx_id: scope.context_id(),
            inner,
            id,
            raw,
            _t: std::marker::PhantomData,
            _no_send: std::marker::PhantomData,
        }
    }
}

impl<T> Drop for Root<T> {
    fn drop(&mut self) {
        // Safety: removing only affects host-side registry.
        self.inner.roots.remove(self.id);
    }
}
