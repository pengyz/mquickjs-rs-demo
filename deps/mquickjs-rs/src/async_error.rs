//! AsyncError: 简化的异步错误处理
//!
//! 核心设计：
//! - 简单错误消息传递
//! - Rust→JS 错误转换
//! - JS→Rust 错误转换

use crate::handles::local::{Local, Value};
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;
use std::ffi::{CStr, CString};
use std::fmt;

/// 异步错误类型
///
/// 简化设计，只支持错误消息字符串
#[derive(Debug, Clone)]
pub enum AsyncError {
    /// 简单错误消息
    Message(String),
}

impl AsyncError {
    /// 获取错误消息
    pub fn message(&self) -> &str {
        match self {
            AsyncError::Message(msg) => msg,
        }
    }

    /// 是否是 JS 异常
    pub fn is_js_exception(&self) -> bool {
        // 当前只支持 Message，不是 JS 异常
        false
    }

    /// 转换为 JS 错误
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    pub fn to_js<'ctx>(&self, scope: &Scope<'ctx>) -> Local<'ctx, Value> {
        let ctx = scope.ctx();
        let msg = self.message();

        unsafe {
            // 使用 JavaScript 创建 Error 对象
            // 手动转义字符串中的特殊字符
            let escaped_msg = msg
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            let js_code = format!("new Error(\"{}\")", escaped_msg);
            let c_js_code = CString::new(js_code.as_str()).unwrap();
            
            // 执行 JavaScript 代码创建 Error 对象
            let result = mquickjs_ffi::JS_Eval(
                ctx,
                c_js_code.as_ptr(),
                js_code.len(),
                CString::new("<async_error>").unwrap().as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            );
            
            scope.value(result)
        }
    }

    /// 从 JS 错误创建 AsyncError
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    pub fn from_js<'ctx>(scope: &Scope<'ctx>, value: Local<'ctx, Value>) -> Self {
        let ctx = scope.ctx();
        let raw = value.as_raw();

        // 尝试获取 error.message
        let msg_key = CString::new("message").unwrap();
        let msg_val = unsafe { mquickjs_ffi::JS_GetPropertyStr(ctx, raw, msg_key.as_ptr()) };

        unsafe {
            if mquickjs_ffi::JS_IsString(ctx, msg_val) != 0 {
                let mut cstr_buf = mquickjs_ffi::JSCStringBuf { buf: [0; 5] };
                let c_str = mquickjs_ffi::JS_ToCString(ctx, msg_val, &mut cstr_buf);
                if !c_str.is_null() {
                    let msg = CStr::from_ptr(c_str)
                        .to_string_lossy()
                        .into_owned();
                    return AsyncError::Message(msg);
                }
            }
        }

        // 如果无法获取 message，使用默认消息
        AsyncError::Message("Unknown error".to_string())
    }
}

impl fmt::Display for AsyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsyncError::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AsyncError {}

impl From<String> for AsyncError {
    fn from(msg: String) -> Self {
        AsyncError::Message(msg)
    }
}

impl From<&str> for AsyncError {
    fn from(msg: &str) -> Self {
        AsyncError::Message(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_error_message() {
        let error = AsyncError::Message("test error".to_string());
        assert_eq!(error.message(), "test error");
        assert!(!error.is_js_exception());
    }

    #[test]
    fn test_async_error_display() {
        let error = AsyncError::Message("test error".to_string());
        assert_eq!(format!("{}", error), "test error");
    }

    #[test]
    fn test_async_error_debug() {
        let error = AsyncError::Message("test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("test error"));
    }

    #[test]
    fn test_async_error_clone() {
        let error = AsyncError::Message("test error".to_string());
        let cloned = error.clone();
        assert_eq!(error.message(), cloned.message());
    }

    #[test]
    fn test_async_error_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AsyncError>();
    }

    #[test]
    fn test_async_error_static() {
        fn assert_static<T: 'static>() {}
        assert_static::<AsyncError>();
    }

    #[test]
    fn test_async_error_from_string() {
        let error: AsyncError = "test error".to_string().into();
        assert_eq!(error.message(), "test error");
    }

    #[test]
    fn test_async_error_from_str() {
        let error: AsyncError = "test error".into();
        assert_eq!(error.message(), "test error");
    }
}