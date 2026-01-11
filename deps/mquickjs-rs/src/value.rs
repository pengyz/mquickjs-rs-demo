use std::ffi::CString;
use std::marker::PhantomData;

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
        mquickjs_ffi::js_is_bool(self.value)
    }

    // 修复 is_function 方法
    pub fn is_function(&self, ctx: &'ctx Context) -> bool {
        unsafe { mquickjs_ffi::JS_IsFunction(ctx.ctx, self.value) != 0 }
    }

    // 添加 is_null 方法
    pub fn is_null(&self, _ctx: &'ctx Context) -> bool {
        // 检查值的标签是否为NULL (JS_TAG_NULL)
        let tag = (self.value as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1);
        tag == mquickjs_ffi::JS_TAG_NULL as u32
    }

    // 添加 is_undefined 方法
    pub fn is_undefined(&self, _ctx: &'ctx Context) -> bool {
        // 检查值的标签是否为undefined (JS_TAG_UNDEFINED)
        let tag = (self.value as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1);
        tag == mquickjs_ffi::JS_TAG_UNDEFINED as u32
    }
}

impl<'ctx> Value<'ctx> {
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.value
    }

    pub fn get_property(&self, ctx: &'ctx Context, name: &str) -> Result<Value<'ctx>, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let result =
            unsafe { mquickjs_ffi::JS_GetPropertyStr(ctx.ctx, self.value, c_name.as_ptr()) };

        if (result as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)
            == mquickjs_ffi::JS_TAG_EXCEPTION as u32
        {
            return Err("Failed to get property".to_string());
        }

        Ok(Value {
            value: result,
            _ctx: PhantomData,
        })
    }

    pub fn set_property(
        &self,
        ctx: &'ctx Context,
        name: &str,
        value: Value<'ctx>,
    ) -> Result<(), String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let _ret = unsafe {
            mquickjs_ffi::JS_SetPropertyStr(ctx.ctx, self.value, c_name.as_ptr(), value.value)
        };

        Ok(())
    }
}
