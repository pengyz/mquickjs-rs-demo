use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_void;

use crate::mquickjs_ffi;
use crate::Value;

pub struct Context {
    pub ctx: *mut mquickjs_ffi::JSContext,
    _memory: Vec<u8>, // 重命名以表明这是未使用的字段
}

impl Context {
    pub fn new(memory_capacity: usize) -> Result<Self, Box<dyn std::error::Error>> {
        // 为JSContext分配内存空间
        let mut memory = vec![0u8; memory_capacity];

        // 加载标准库定义
        extern "C" {
            static js_stdlib: mquickjs_ffi::JSSTDLibraryDef;
        }

        // 安全地访问静态变量
        let stdlib_def = unsafe { js_stdlib };

        // 创建新的JSContext
        let ctx = unsafe {
            mquickjs_ffi::JS_NewContext(
                memory.as_mut_ptr() as *mut c_void,
                memory.len(),
                &stdlib_def,
            )
        };

        if ctx.is_null() {
            return Err("Failed to create JSContext".into());
        }

        Ok(Context {
            ctx,
            _memory: memory,
        })
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        let c_code = CString::new(code).map_err(|e| e.to_string())?;
        let filename = CString::new("eval.js").unwrap();

        let result = unsafe {
            mquickjs_ffi::JS_Eval(
                self.ctx,
                c_code.as_ptr(),
                code.len(),
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };

        // 检查返回值是否为异常
        // 在mquickjs中，JS_TAG_EXCEPTION是特殊的tag
        let tag = (result as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1);
        if tag == mquickjs_ffi::JS_TAG_EXCEPTION as u32 {
            let exception = unsafe { mquickjs_ffi::JS_GetException(self.ctx) };

            // 创建一个临时缓冲区用于JS_ToCString
            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let error_ptr =
                unsafe { mquickjs_ffi::JS_ToCString(self.ctx, exception, &mut cstr_buf) };

            if !error_ptr.is_null() {
                let error_str = unsafe { CStr::from_ptr(error_ptr).to_string_lossy().into_owned() };

                return Err(error_str);
            } else {
                return Err("Unknown error".to_string());
            }
        } else {
            // 创建一个临时缓冲区用于JS_ToCString
            let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
            let result_ptr = unsafe { mquickjs_ffi::JS_ToCString(self.ctx, result, &mut cstr_buf) };

            if !result_ptr.is_null() {
                let result_str =
                    unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };

                Ok(result_str)
            } else {
                Ok("undefined".to_string())
            }
        }
    }

    /// 创建一个新的字符串值
    pub fn create_string(&self, rust_str: &str) -> Result<Value, String> {
        let c_str = CString::new(rust_str).map_err(|e| e.to_string())?;
        let js_value = unsafe { mquickjs_ffi::JS_NewString(self.ctx, c_str.as_ptr()) };

        if (js_value as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)
            == mquickjs_ffi::JS_TAG_EXCEPTION as u32
        {
            return Err("Failed to create string".to_string());
        }

        Ok(Value {
            value: js_value,
            _ctx: PhantomData,
        })
    }

    /// 创建一个新的数字值
    pub fn create_number(&self, num: f64) -> Result<Value, String> {
        let js_value = unsafe { mquickjs_ffi::JS_NewFloat64(self.ctx, num) };

        if (js_value as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)
            == mquickjs_ffi::JS_TAG_EXCEPTION as u32
        {
            return Err("Failed to create number".to_string());
        }

        Ok(Value {
            value: js_value,
            _ctx: PhantomData,
        })
    }

    /// 创建一个新的布尔值
    pub fn create_boolean(&self, boolean: bool) -> Result<Value, String> {
        // 使用 JS_VALUE_MAKE_SPECIAL 创建布尔值，这是 JS_NewBool 的实际实现
        let js_value = if boolean {
            0x07 // JS_TRUE (JS_TAG_BOOL with value 1)
        } else {
            0x03 // JS_FALSE (JS_TAG_BOOL with value 0)
        };

        Ok(Value {
            value: js_value,
            _ctx: PhantomData,
        })
    }

    /// 创建一个新的对象
    pub fn create_object(&self) -> Result<Value, String> {
        let obj = unsafe { mquickjs_ffi::JS_NewObject(self.ctx) };
        if (obj as u32) & ((1 << mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)
            == mquickjs_ffi::JS_TAG_EXCEPTION as u32
        {
            return Err("Failed to create object".to_string());
        }
        Ok(Value {
            value: obj,
            _ctx: PhantomData,
        })
    }

    /// 将值转换为Rust字符串
    pub fn get_string(&self, value: Value) -> Result<String, String> {
        if !value.is_string(self) {
            return Err("Value is not a string".to_string());
        }

        let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
        let result_ptr =
            unsafe { mquickjs_ffi::JS_ToCString(self.ctx, value.value, &mut cstr_buf) };

        if !result_ptr.is_null() {
            let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy().into_owned() };
            Ok(result_str)
        } else {
            Err("Failed to convert Value to string".to_string())
        }
    }

    /// 获取数字值
    pub fn get_number(&self, value: Value) -> Result<f64, String> {
        if !value.is_number(self) {
            return Err("Value is not a number".to_string());
        }

        let mut result = 0.0;
        let ret = unsafe { mquickjs_ffi::JS_ToNumber(self.ctx, &mut result, value.value) };

        if ret != 0 {
            return Err("Failed to convert Value to number".to_string());
        }

        Ok(result)
    }

    /// 获取布尔值
    pub fn get_boolean(&self, value: Value) -> Result<bool, String> {
        if !value.is_bool(self) {
            return Err("Value is not a boolean".to_string());
        }

        // mquickjs 中没有 JS_ToBool，我们需要使用 JS_ToInt32 然后转换
        let mut result = 0i32;
        let ret = unsafe { mquickjs_ffi::JS_ToInt32(self.ctx, &mut result, value.value) };

        if ret != 0 {
            return Err("Failed to convert Value to boolean".to_string());
        }

        Ok(result != 0)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            mquickjs_ffi::JS_FreeContext(self.ctx);
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(1024 * 1024).expect("Failed to create default Context") // 默认 1MB
    }
}
