use std::marker::PhantomData;

use crate::handles::scope::{ContextId, Scope};
use crate::mquickjs_ffi;

pub struct Value;
pub struct Object;
pub struct Function;
pub struct Array;

#[derive(Copy, Clone)]
pub struct Local<'ctx, T = Value> {
    raw: mquickjs_ffi::JSValue,
    ctx_id: ContextId,
    _m: PhantomData<&'ctx crate::context::Context>,
    _t: PhantomData<T>,
}

impl<'ctx, T> Local<'ctx, T> {
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }

    pub fn ctx_id(&self) -> ContextId {
        self.ctx_id
    }

    pub(crate) fn from_raw_for_same_ctx(raw: mquickjs_ffi::JSValue) -> Self {
        // Safety/semantic: type marker changes do not change the underlying JSValue.
        // The ctx_id will be overwritten by callers that already validated same-context.
        // Here we keep it as a dummy and rely on `transmute`-free reconstruction below.
        Self {
            raw,
            ctx_id: ContextId(0),
            _m: PhantomData,
            _t: PhantomData,
        }
    }

    pub(crate) fn with_ctx_id(mut self, ctx_id: ContextId) -> Self {
        self.ctx_id = ctx_id;
        self
    }
}

impl<'ctx> Scope<'ctx> {
    pub fn value(&self, raw: mquickjs_ffi::JSValue) -> Local<'ctx, Value> {
        Local {
            raw,
            ctx_id: self.context_id(),
            _m: PhantomData,
            _t: PhantomData,
        }
    }
}
