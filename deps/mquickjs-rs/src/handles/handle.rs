use std::marker::PhantomData;

use crate::handles::scope::{ContextId, Scope};
use crate::mquickjs_ffi;

/// A V8-style handle whose lifetime is tied to a specific handle scope.
///
/// Unlike `Local<'ctx, T>`, `Handle<'hs, 'ctx, T>` cannot outlive the handle
/// scope that created it, which lets us enforce "non-escaped values must not
/// leave their scope" at compile time.
#[derive(Copy, Clone)]
pub struct Handle<'hs, 'ctx, T = crate::handles::local::Value> {
    raw: mquickjs_ffi::JSValue,
    ctx_id: ContextId,
    _hs: PhantomData<&'hs ()>,
    _ctx: PhantomData<&'ctx Scope<'ctx>>,
    _t: PhantomData<T>,
}

impl<'hs, 'ctx, T> Handle<'hs, 'ctx, T> {
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }

    pub fn ctx_id(&self) -> ContextId {
        self.ctx_id
    }

    pub(crate) fn from_raw(raw: mquickjs_ffi::JSValue, ctx_id: ContextId) -> Self {
        Self {
            raw,
            ctx_id,
            _hs: PhantomData,
            _ctx: PhantomData,
            _t: PhantomData,
        }
    }
}
