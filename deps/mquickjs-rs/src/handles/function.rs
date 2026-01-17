use std::ffi::CStr;

use crate::handles::local::{Function, Local, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;

impl<'ctx> Local<'ctx, Value> {
    pub fn is_function(&self, scope: &Scope<'ctx>) -> bool {
        (unsafe { mquickjs_ffi::JS_IsFunction(scope.ctx(), self.as_raw()) }) != 0
    }

    pub fn try_into_function(self, scope: &Scope<'ctx>) -> Result<Local<'ctx, Function>, String> {
        if self.is_function(scope) {
            Ok(Local::from_raw_for_same_ctx(self.as_raw()).with_ctx_id(self.ctx_id()))
        } else {
            Err("Value is not a function".to_string())
        }
    }
}

impl<'ctx> Local<'ctx, Function> {
    pub fn call(
        &self,
        scope: &Scope<'ctx>,
        this_val: Local<'ctx, Value>,
        args: &[Local<'ctx, Value>],
    ) -> Result<Local<'ctx, Value>, String> {
        if unsafe { mquickjs_ffi::JS_StackCheck(scope.ctx(), (args.len() + 2) as u32) } != 0 {
            return Err("Stack overflow".to_string());
        }

        for arg in args.iter().rev() {
            unsafe { mquickjs_ffi::JS_PushArg(scope.ctx(), arg.as_raw()) };
        }

        unsafe { mquickjs_ffi::JS_PushArg(scope.ctx(), self.as_raw()) };
        unsafe { mquickjs_ffi::JS_PushArg(scope.ctx(), this_val.as_raw()) };

        let result = unsafe { mquickjs_ffi::JS_Call(scope.ctx(), args.len() as i32) };

        if (result as u32) & ((1u32 << (mquickjs_ffi::JS_TAG_SPECIAL_BITS as u32)) - 1)
            == (mquickjs_ffi::JS_TAG_EXCEPTION as u32)
        {
            let exception = unsafe { mquickjs_ffi::JS_GetException(scope.ctx()) };

            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let error_ptr = unsafe { mquickjs_ffi::JS_ToCString(scope.ctx(), exception, &mut cstr_buf) };

            if !error_ptr.is_null() {
                let error_str = unsafe { CStr::from_ptr(error_ptr).to_string_lossy().into_owned() };
                return Err(error_str);
            }
            return Err("Unknown error during function call".to_string());
        }

        Ok(scope.value(result))
    }

}
