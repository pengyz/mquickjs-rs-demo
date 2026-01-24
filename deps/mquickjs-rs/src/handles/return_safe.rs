use crate::handles::local::{Local, Value};
use crate::handles::scope::{ContextId, Scope};
use crate::mquickjs_ffi;
use std::marker::PhantomData;

/// A value that is safe to be returned across the native<->JS boundary.
///
/// Semantics:
/// - This is an **owned**, context-bound handle that does not expose raw engine APIs.
/// - It is **not** a persistent root (use `Global<T>` for cross-call storage).
/// - The actual "pinning" for the return boundary is performed by glue using the current `Env`.
pub struct ReturnSafe<T> {
    raw: mquickjs_ffi::JSValue,
    ctx_id: ContextId,
    _marker: PhantomData<T>,
}

impl<T> ReturnSafe<T> {
    pub fn ctx_id(&self) -> ContextId {
        self.ctx_id
    }

    pub fn to_local<'ctx>(&self, scope: &Scope<'ctx>) -> Local<'ctx, T> {
        assert_eq!(
            scope.context_id(),
            self.ctx_id,
            "cross-context ReturnSafe::to_local",
        );
        // Safety/semantic: the caller must ensure the value is reachable when used.
        // Return boundary pinning is handled by glue via `Env`.
        Local::from_raw_for_same_ctx(self.raw).with_ctx_id(self.ctx_id)
    }

    pub(crate) fn raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }

    pub(crate) fn new(raw: mquickjs_ffi::JSValue, ctx_id: ContextId) -> Self {
        Self {
            raw,
            ctx_id,
            _marker: PhantomData,
        }
    }
}

pub type ReturnAny = ReturnSafe<Value>;
