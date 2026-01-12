use std::cell::UnsafeCell;
use std::ffi::CString;
use std::marker::PhantomData;

use crate::mquickjs_ffi;
use crate::Context;

/// Borrowed view of a JSValue tied to a Context lifetime.
///
/// NOTE: mquickjs does *not* use refcounted JSValue. If you need to keep a value
/// across calls/GC, convert it to [`PinnedValue`] (GCRef-based pin/unpin).
#[derive(Copy, Clone)]
pub struct ValueRef<'ctx> {
    value: mquickjs_ffi::JSValue,
    _ctx: PhantomData<&'ctx Context>,
}

impl<'ctx> ValueRef<'ctx> {
    pub fn new(value: mquickjs_ffi::JSValue) -> Self {
        Self {
            value,
            _ctx: PhantomData,
        }
    }

    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.value
    }

    pub fn is_string(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsString(ctx.ctx, self.value) != 0 }
    }

    pub fn is_number(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsNumber(ctx.ctx, self.value) != 0 }
    }

    pub fn is_bool(&self, _ctx: &'ctx Context) -> bool {
        mquickjs_ffi::js_is_bool(self.value)
    }

    pub fn is_function(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsFunction(ctx.ctx, self.value) != 0 }
    }

    pub fn is_null(&self, _ctx: &'ctx Context) -> bool {
        let tag = (self.value as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1);
        tag == (mquickjs_ffi::JS_TAG_NULL as u32)
    }

    pub fn is_undefined(&self, _ctx: &'ctx Context) -> bool {
        let tag = (self.value as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1);
        tag == (mquickjs_ffi::JS_TAG_UNDEFINED as u32)
    }

    pub fn get_property(&self, ctx: &'ctx Context, name: &str) -> Result<ValueRef<'ctx>, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let result =
            unsafe { mquickjs_ffi::JS_GetPropertyStr(ctx.ctx, self.value, c_name.as_ptr()) };

        if (result as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            return Err("Failed to get property".to_string());
        }

        Ok(ValueRef::new(result))
    }

    pub fn set_property(
        &self,
        ctx: &'ctx Context,
        name: &str,
        value: ValueRef<'ctx>,
    ) -> Result<(), String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        unsafe {
            mquickjs_ffi::JS_SetPropertyStr(ctx.ctx, self.value, c_name.as_ptr(), value.value);
        }
        Ok(())
    }

    pub fn pin(&self, ctx: &'ctx Context) -> PinnedValue<'ctx> {
        PinnedValue::new(ctx, self.value)
    }
}

/// GC-rooted JS value (pin/unpin via mquickjs GCRef list).
///
/// This allows holding a JS value across calls/GC, but it must not outlive the
/// underlying Context.
pub struct PinnedValue<'ctx> {
    ctx: *mut mquickjs_ffi::JSContext,
    gc_ref: UnsafeCell<mquickjs_ffi::JSGCRef>,
    _ctx: PhantomData<&'ctx Context>,
}

impl<'ctx> PinnedValue<'ctx> {
    pub fn new(ctx: &'ctx Context, value: mquickjs_ffi::JSValue) -> Self {
        let p = Self {
            ctx: ctx.ctx,
            gc_ref: UnsafeCell::new(mquickjs_ffi::JSGCRef {
                val: mquickjs_ffi::JS_UNDEFINED,
                prev: std::ptr::null_mut(),
            }),
            _ctx: PhantomData,
        };

        // Safety: gc_ref lives as long as self; ctx is alive for 'ctx.
        unsafe {
            let slot = mquickjs_ffi::JS_AddGCRef(ctx.ctx, p.gc_ref.get());
            *slot = value;
        }

        p
    }

    pub fn as_ref(&self) -> ValueRef<'ctx> {
        // Safety: gc_ref is pinned and owned by self.
        let v = unsafe { (*self.gc_ref.get()).val };
        ValueRef::new(v)
    }

    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        unsafe { (*self.gc_ref.get()).val }
    }
}

impl Drop for PinnedValue<'_> {
    fn drop(&mut self) {
        // Safety: ctx is the one used in new(); gc_ref was added to ctx list.
        unsafe {
            mquickjs_ffi::JS_DeleteGCRef(self.ctx, self.gc_ref.get());
        }
    }
}

/// Back-compat alias: most existing code used `Value<'ctx>` as a borrowed JSValue.
/// Prefer using [`ValueRef`] explicitly in new code.
pub type Value<'ctx> = ValueRef<'ctx>;
