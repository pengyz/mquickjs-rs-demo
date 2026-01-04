use std::marker::PhantomData;
use std::ffi::CString;

use crate::mquickjs_ffi;
use crate::Context;

/// 修改Value结构体以包含PhantomData
#[derive(Copy, Clone)]
pub struct Value<'ctx> {
    pub value: mquickjs_ffi::JSValue,
    pub _ctx: PhantomData<&'ctx Context>,
}

impl<'ctx> Value<'ctx> {
    // 修复 is_string 方法
    pub fn is_string(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsString(ctx.ctx, self.value) != 0 }
    }
    
    // 修复 is_number 方法
    pub fn is_number(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsNumber(ctx.ctx, self.value) != 0 }
    }
    
    // 修复 is_bool 方法 - 检查值是否为布尔类型
    pub fn is_bool(&self, _ctx: &'ctx Context) -> bool {
        // 检查值的标签是否为布尔类型 (JS_TAG_BOOL)
        let tag = (self.value as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1);
        tag == 0x03  // JS_TAG_BOOL (JS_TAG_SPECIAL | (0 << 2)) = 3
    }
    
    // 修复 is_function 方法
    pub fn is_function(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsFunction(ctx.ctx, self.value) != 0 }
    }
}

impl<'ctx> Value<'ctx> {
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.value
    }

    pub fn get_property(&self, ctx: &'ctx Context, name: &str) -> Result<Value<'ctx>, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let result = unsafe { 
            mquickjs_ffi::JS_GetPropertyStr(ctx.ctx, self.value, c_name.as_ptr()) 
        };
        
        if (result as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1) == mquickjs_ffi::JS_TAG_EXCEPTION as u32 {
            return Err("Failed to get property".to_string());
        }
        
        Ok(Value {
            value: result,
            _ctx: PhantomData,
        })
    }

    pub fn set_property(&self, ctx: &'ctx Context, name: &str, value: Value<'ctx>) -> Result<(), String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let _ret = unsafe {
            mquickjs_ffi::JS_SetPropertyStr(ctx.ctx, self.value, c_name.as_ptr(), value.value)
        };
        
        Ok(())
    }
}