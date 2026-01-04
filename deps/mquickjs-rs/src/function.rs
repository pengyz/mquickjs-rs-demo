use std::marker::PhantomData;
use std::ffi::CStr;

use crate::mquickjs_ffi;
use crate::value::Value;
use crate::Context;

// 添加 Function 结构体定义
pub struct Function<'ctx> {
    value: Value<'ctx>,
}

impl<'ctx> Function<'ctx> {
    pub fn call(&self, ctx: &'ctx Context, this_val: Value<'ctx>, args: &[Value<'ctx>]) -> Result<Value<'ctx>, String> {
        // 检查是否为函数
        if !self.value.is_function(ctx) {
            return Err("Value is not a function".to_string());
        }

        // 检查栈空间
        if unsafe { mquickjs_ffi::JS_StackCheck(ctx.ctx, (args.len() + 2) as u32) } != 0 {
            return Err("Stack overflow".to_string());
        }

        // 将参数压入栈（逆序）
        for arg in args.iter().rev() {
            unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, arg.value) };
        }
        
        // 压入函数
        unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, self.value.value) };
        
        // 压入this值
        unsafe { mquickjs_ffi::JS_PushArg(ctx.ctx, this_val.value) };

        // 调用函数
        let result = unsafe { mquickjs_ffi::JS_Call(ctx.ctx, args.len() as i32) };

        // 检查结果
        if (result as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1) == mquickjs_ffi::JS_TAG_EXCEPTION as u32 {
            let exception = unsafe { mquickjs_ffi::JS_GetException(ctx.ctx) };
            
            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let error_ptr = unsafe { 
                mquickjs_ffi::JS_ToCString(ctx.ctx, exception, &mut cstr_buf) 
            };
            
            if !error_ptr.is_null() {
                let error_str = unsafe {
                    CStr::from_ptr(error_ptr).to_string_lossy().into_owned()
                };
                
                return Err(error_str);
            } else {
                return Err("Unknown error during function call".to_string());
            }
        }

        Ok(Value {
            value: result,
            _ctx: PhantomData,
        })
    }
}