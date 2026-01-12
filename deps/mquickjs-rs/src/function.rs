use std::ffi::CStr;

use crate::mquickjs_ffi;
use crate::value::ValueRef;
use crate::Context;

pub struct Function<'ctx> {
    value: ValueRef<'ctx>,
}

impl<'ctx> Function<'ctx> {
    pub fn call(
        &self,
        ctx: &'ctx Context,
        this_val: ValueRef<'ctx>,
        args: &[ValueRef<'ctx>],
    ) -> Result<ValueRef<'ctx>, String> {
        if !self.value.is_function(ctx) {
            return Err("Value is not a function".to_string());
        }

        if unsafe { mquickjs_ffi::JS_StackCheck(ctx.ctx, (args.len() + 2) as u32) } != 0 {
            return Err("Stack overflow".to_string());
        }

        for arg in args.iter().rev() {
            unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, arg.as_raw()) };
        }

        unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, self.value.as_raw()) };
        unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, this_val.as_raw()) };

        let result = unsafe { mquickjs_ffi::JS_Call(ctx.ctx, args.len() as i32) };

        if (result as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            let exception = unsafe { mquickjs_ffi::JS_GetException(ctx.ctx) };

            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let error_ptr = unsafe { mquickjs_ffi::JS_ToCString(ctx.ctx, exception, &mut cstr_buf) };

            if !error_ptr.is_null() {
                let error_str = unsafe { CStr::from_ptr(error_ptr).to_string_lossy().into_owned() };
                return Err(error_str);
            }
            return Err("Unknown error during function call".to_string());
        }

        Ok(ValueRef::new(result))
    }
}
