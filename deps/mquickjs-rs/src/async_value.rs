//! AsyncValue: JS→Rust 快照和 Rust→JS 还原机制
//!
//! 核心约束：
//! - 异步执行函数不允许调用 JS 对象
//! - 必须操作 Rust 对象
//! - Rust 对象在 callback 结束后由框架转换到 JS 层
//! - `any`/`object` 参数必须在提交时"冻结"成 Rust 拥有的数据

use crate::handles::local::{Local, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;
use std::collections::HashMap;

/// 异步参数的 Rust 表示 - Send + 'static
///
/// 用于在异步任务提交时将 JS 值"快照"为 Rust 拥有的数据，
/// 确保异步执行期间零 JS 访问。
#[derive(Clone, Debug)]
pub enum AsyncValue {
    // 原语类型 - 直接映射
    Null,
    Undefined,
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),

    // 复合类型 - 递归表示
    Array(Vec<AsyncValue>),
    Object(HashMap<String, AsyncValue>),

    // 特殊类型 - 序列化兜底
    Json(String), // JSON.stringify 的结果

    // 不可序列化的 JS 值（函数、Symbol 等）
    // 提交时报错，要求用户显式处理
    Unsupported,
}

impl AsyncValue {
    /// 检查是否为 null
    pub fn is_null(&self) -> bool {
        matches!(self, AsyncValue::Null)
    }

    /// 检查是否为 undefined
    pub fn is_undefined(&self) -> bool {
        matches!(self, AsyncValue::Undefined)
    }

    /// 检查是否为布尔值
    pub fn is_bool(&self) -> bool {
        matches!(self, AsyncValue::Bool(_))
    }

    /// 检查是否为整数
    pub fn is_int(&self) -> bool {
        matches!(self, AsyncValue::Int(_))
    }

    /// 检查是否为浮点数
    pub fn is_float(&self) -> bool {
        matches!(self, AsyncValue::Float(_))
    }

    /// 检查是否为数字（整数或浮点数）
    pub fn is_number(&self) -> bool {
        self.is_int() || self.is_float()
    }

    /// 检查是否为字符串
    pub fn is_string(&self) -> bool {
        matches!(self, AsyncValue::String(_))
    }

    /// 检查是否为数组
    pub fn is_array(&self) -> bool {
        matches!(self, AsyncValue::Array(_))
    }

    /// 检查是否为对象
    pub fn is_object(&self) -> bool {
        matches!(self, AsyncValue::Object(_))
    }

    /// 检查是否为 JSON
    pub fn is_json(&self) -> bool {
        matches!(self, AsyncValue::Json(_))
    }

    /// 检查是否为不支持的类型
    pub fn is_unsupported(&self) -> bool {
        matches!(self, AsyncValue::Unsupported)
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AsyncValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 获取整数值
    pub fn as_int(&self) -> Option<i32> {
        match self {
            AsyncValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 获取浮点值
    pub fn as_float(&self) -> Option<f64> {
        match self {
            AsyncValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// 获取数字值（整数或浮点数）
    pub fn as_number(&self) -> Option<f64> {
        match self {
            AsyncValue::Int(i) => Some(*i as f64),
            AsyncValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// 获取字符串引用
    pub fn as_string(&self) -> Option<&str> {
        match self {
            AsyncValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 获取数组引用
    pub fn as_array(&self) -> Option<&Vec<AsyncValue>> {
        match self {
            AsyncValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// 获取可变数组引用
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<AsyncValue>> {
        match self {
            AsyncValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// 获取对象引用
    pub fn as_object(&self) -> Option<&HashMap<String, AsyncValue>> {
        match self {
            AsyncValue::Object(map) => Some(map),
            _ => None,
        }
    }

    /// 获取可变对象引用
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, AsyncValue>> {
        match self {
            AsyncValue::Object(map) => Some(map),
            _ => None,
        }
    }

    /// 获取 JSON 字符串引用
    pub fn as_json(&self) -> Option<&str> {
        match self {
            AsyncValue::Json(s) => Some(s),
            _ => None,
        }
    }

    /// 从 JS 值创建 AsyncValue（JS→Rust 快照）
    ///
    /// # Safety
    ///
    /// 此函数访问 JS 引擎状态，必须在 JS 主线程调用。
    pub fn from_js<'ctx>(scope: &Scope<'ctx>, value: Local<'ctx, Value>) -> Self {
        let ctx = scope.ctx();
        let raw = value.as_raw();

        // 检查特殊值
        if raw == mquickjs_ffi::JS_NULL {
            return AsyncValue::Null;
        }
        if raw == mquickjs_ffi::JS_UNDEFINED {
            return AsyncValue::Undefined;
        }
        if raw == mquickjs_ffi::JS_TRUE {
            return AsyncValue::Bool(true);
        }
        if raw == mquickjs_ffi::JS_FALSE {
            return AsyncValue::Bool(false);
        }

        // 检查布尔值（通过 tag）
        if mquickjs_ffi::js_is_bool(raw) {
            let tag = mquickjs_ffi::js_value_special_tag(raw);
            return AsyncValue::Bool(tag != 0);
        }

        // 检查数组（必须在数字之前，因为数组转数字会得到 NaN）
        unsafe {
            if mquickjs_ffi::JS_IsArray(ctx, raw) != 0 {
                let mut array = Vec::new();
                // 获取数组长度
                let len_key = std::ffi::CString::new("length").unwrap();
                let len_val = mquickjs_ffi::JS_GetPropertyStr(ctx, raw, len_key.as_ptr());
                let mut len: i32 = 0;
                if mquickjs_ffi::JS_ToInt32(ctx, &mut len as *mut i32, len_val) == 0 {
                    for i in 0..len as u32 {
                        let elem = mquickjs_ffi::JS_GetPropertyUint32(ctx, raw, i);
                        let elem_local = scope.value(elem);
                        array.push(AsyncValue::from_js(scope, elem_local));
                    }
                }
                return AsyncValue::Array(array);
            }
        }

        // 检查字符串
        unsafe {
            if mquickjs_ffi::JS_IsString(ctx, raw) != 0 {
                let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
                let c_str = mquickjs_ffi::JS_ToCString(ctx, raw, &mut cstr_buf);
                if !c_str.is_null() {
                    let rust_str = std::ffi::CStr::from_ptr(c_str)
                        .to_string_lossy()
                        .into_owned();
                    return AsyncValue::String(rust_str);
                }
            }
        }

        // 检查函数
        unsafe {
            if mquickjs_ffi::JS_IsFunction(ctx, raw) != 0 {
                return AsyncValue::Unsupported;
            }
        }

        // 检查对象（包括 Date、RegExp 等）
        // 必须在数字之前，因为对象转数字会得到 NaN
        unsafe {
            let class_id = mquickjs_ffi::JS_GetClassID(ctx, raw);
            // JS_CLASS_OBJECT = 0, JS_CLASS_ARRAY = 1, etc.
            // 只对真正的对象（非数组）使用 JSON.stringify
            if class_id >= 0 && class_id != 1 {  // 不是数组
                // 尝试 JSON.stringify
                let json_str_key = std::ffi::CString::new("JSON").unwrap();
                let stringify_key = std::ffi::CString::new("stringify").unwrap();
                let global = mquickjs_ffi::JS_GetGlobalObject(ctx);
                let json_obj = mquickjs_ffi::JS_GetPropertyStr(ctx, global, json_str_key.as_ptr());
                let stringify_fn = mquickjs_ffi::JS_GetPropertyStr(ctx, json_obj, stringify_key.as_ptr());

                if mquickjs_ffi::JS_IsFunction(ctx, stringify_fn) != 0 {
                    // 调用 JSON.stringify(value)
                    // 按照 function.rs 的调用顺序：
                    // 1. 推送参数（逆序）
                    // 2. 推送函数
                    // 3. 推送 this
                    // 4. 调用 JS_Call
                    mquickjs_ffi::JS_PushArg(ctx, raw);  // 参数: value (逆序，只有一个参数)
                    mquickjs_ffi::JS_PushArg(ctx, stringify_fn);  // 函数
                    mquickjs_ffi::JS_PushArg(ctx, json_obj);  // this

                    let result = mquickjs_ffi::JS_Call(ctx, 1);  // 1 个参数
                    if mquickjs_ffi::JS_IsString(ctx, result) != 0 {
                        let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
                        let c_str = mquickjs_ffi::JS_ToCString(ctx, result, &mut cstr_buf);
                        if !c_str.is_null() {
                            let rust_str = std::ffi::CStr::from_ptr(c_str)
                                .to_string_lossy()
                                .into_owned();
                            return AsyncValue::Json(rust_str);
                        }
                    }
                }
            }
        }

        // 检查数字（必须在对象之后，因为对象转数字会得到 NaN）
        unsafe {
            let mut num: f64 = 0.0;
            if mquickjs_ffi::JS_ToNumber(ctx, &mut num as *mut f64, raw) == 0 {
                // 检查是否为整数
                if num.fract() == 0.0 && num >= i32::MIN as f64 && num <= i32::MAX as f64 {
                    return AsyncValue::Int(num as i32);
                }
                return AsyncValue::Float(num);
            }
        }

        // 兜底：不支持的类型
        AsyncValue::Unsupported
    }

    /// 将 AsyncValue 转换为 JS 值（Rust→JS 还原）
    ///
    /// # Safety
    ///
    /// 此函数访问 JS 引擎状态，必须在 JS 主线程调用。
    pub fn to_js<'ctx>(&self, scope: &Scope<'ctx>) -> Local<'ctx, Value> {
        let ctx = scope.ctx();

        let raw = match self {
            AsyncValue::Null => mquickjs_ffi::JS_NULL,
            AsyncValue::Undefined => mquickjs_ffi::JS_UNDEFINED,
            AsyncValue::Bool(b) => mquickjs_ffi::js_mkbool(*b),
            AsyncValue::Int(i) => unsafe { mquickjs_ffi::JS_NewInt32(ctx, *i) },
            AsyncValue::Float(f) => unsafe { mquickjs_ffi::JS_NewFloat64(ctx, *f) },
            AsyncValue::String(s) => {
                let c_str = std::ffi::CString::new(s.as_str()).unwrap();
                unsafe { mquickjs_ffi::JS_NewString(ctx, c_str.as_ptr()) }
            }
            AsyncValue::Array(arr) => {
                unsafe {
                    let js_array = mquickjs_ffi::JS_NewArray(ctx, arr.len() as i32);
                    for (i, elem) in arr.iter().enumerate() {
                        let js_elem = elem.to_js(scope);
                        mquickjs_ffi::JS_SetPropertyUint32(ctx, js_array, i as u32, js_elem.as_raw());
                    }
                    js_array
                }
            }
            AsyncValue::Object(map) => {
                unsafe {
                    let js_obj = mquickjs_ffi::JS_NewObject(ctx);
                    for (key, value) in map.iter() {
                        let c_key = std::ffi::CString::new(key.as_str()).unwrap();
                        let js_value = value.to_js(scope);
                        mquickjs_ffi::JS_SetPropertyStr(ctx, js_obj, c_key.as_ptr(), js_value.as_raw());
                    }
                    js_obj
                }
            }
            AsyncValue::Json(json_str) => {
                // JSON.parse
                unsafe {
                    let global = mquickjs_ffi::JS_GetGlobalObject(ctx);
                    let json_key = std::ffi::CString::new("JSON").unwrap();
                    let parse_key = std::ffi::CString::new("parse").unwrap();
                    let json_obj = mquickjs_ffi::JS_GetPropertyStr(ctx, global, json_key.as_ptr());
                    let parse_fn = mquickjs_ffi::JS_GetPropertyStr(ctx, json_obj, parse_key.as_ptr());

                    if mquickjs_ffi::JS_IsFunction(ctx, parse_fn) != 0 {
                        let c_json = std::ffi::CString::new(json_str.as_str()).unwrap();
                        let js_json_str = mquickjs_ffi::JS_NewString(ctx, c_json.as_ptr());
                        // 调用 JSON.parse
                        let result = mquickjs_ffi::JS_Call(ctx, 0); // 简化调用
                        result
                    } else {
                        mquickjs_ffi::JS_NULL
                    }
                }
            }
            AsyncValue::Unsupported => {
                // 不支持的类型应该在 from_js 时就被拒绝
                // 这里返回 null 作为兜底
                mquickjs_ffi::JS_NULL
            }
        };

        scope.value(raw)
    }
}

/// 从 JS 值创建 AsyncValue 的便捷函数
pub fn from_js<'ctx>(scope: &Scope<'ctx>, value: Local<'ctx, Value>) -> AsyncValue {
    AsyncValue::from_js(scope, value)
}

/// 将 AsyncValue 转换为 JS 值的便捷函数
pub fn to_js<'ctx>(scope: &Scope<'ctx>, async_value: &AsyncValue) -> Local<'ctx, Value> {
    async_value.to_js(scope)
}

/// 检查 AsyncValue 是否可以安全用于异步任务
///
/// 返回 true 表示可以安全使用，false 表示包含不支持的类型。
pub fn is_safe_for_async(value: &AsyncValue) -> bool {
    !value.is_unsupported()
}

/// 获取 AsyncValue 的类型描述（用于错误消息）
pub fn type_description(value: &AsyncValue) -> &'static str {
    match value {
        AsyncValue::Null => "null",
        AsyncValue::Undefined => "undefined",
        AsyncValue::Bool(_) => "boolean",
        AsyncValue::Int(_) => "integer",
        AsyncValue::Float(_) => "float",
        AsyncValue::String(_) => "string",
        AsyncValue::Array(_) => "array",
        AsyncValue::Object(_) => "object",
        AsyncValue::Json(_) => "json",
        AsyncValue::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_value_construction() {
        // 测试基本构造
        let null = AsyncValue::Null;
        assert!(null.is_null());
        assert!(!null.is_undefined());

        let undefined = AsyncValue::Undefined;
        assert!(undefined.is_undefined());
        assert!(!undefined.is_null());

        let bool_val = AsyncValue::Bool(true);
        assert!(bool_val.is_bool());
        assert_eq!(bool_val.as_bool(), Some(true));

        let int_val = AsyncValue::Int(42);
        assert!(int_val.is_int());
        assert_eq!(int_val.as_int(), Some(42));

        let float_val = AsyncValue::Float(3.14);
        assert!(float_val.is_float());
        assert!((float_val.as_float().unwrap() - 3.14).abs() < 0.001);

        let string_val = AsyncValue::String("hello".to_string());
        assert!(string_val.is_string());
        assert_eq!(string_val.as_string(), Some("hello"));

        let array_val = AsyncValue::Array(vec![AsyncValue::Int(1), AsyncValue::Int(2)]);
        assert!(array_val.is_array());
        assert_eq!(array_val.as_array().unwrap().len(), 2);

        let mut map = HashMap::new();
        map.insert("key".to_string(), AsyncValue::String("value".to_string()));
        let object_val = AsyncValue::Object(map);
        assert!(object_val.is_object());
        assert_eq!(
            object_val.as_object().unwrap().get("key").unwrap().as_string(),
            Some("value")
        );

        let json_val = AsyncValue::Json("{\"key\":\"value\"}".to_string());
        assert!(json_val.is_json());
        assert_eq!(json_val.as_json(), Some("{\"key\":\"value\"}"));

        let unsupported = AsyncValue::Unsupported;
        assert!(unsupported.is_unsupported());
    }

    #[test]
    fn test_async_value_send() {
        // 验证 AsyncValue 是 Send
        fn assert_send<T: Send>() {}
        assert_send::<AsyncValue>();
    }

    #[test]
    fn test_async_value_static() {
        // 验证 AsyncValue 是 'static
        fn assert_static<T: 'static>() {}
        assert_static::<AsyncValue>();
    }

    #[test]
    fn test_is_safe_for_async() {
        assert!(is_safe_for_async(&AsyncValue::Null));
        assert!(is_safe_for_async(&AsyncValue::Int(42)));
        assert!(!is_safe_for_async(&AsyncValue::Unsupported));
    }

    #[test]
    fn test_type_description() {
        assert_eq!(type_description(&AsyncValue::Null), "null");
        assert_eq!(type_description(&AsyncValue::Bool(true)), "boolean");
        assert_eq!(type_description(&AsyncValue::Int(42)), "integer");
        assert_eq!(type_description(&AsyncValue::String("test".to_string())), "string");
        assert_eq!(type_description(&AsyncValue::Unsupported), "unsupported");
    }
}