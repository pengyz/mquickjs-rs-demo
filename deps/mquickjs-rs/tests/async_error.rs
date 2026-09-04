//! AsyncError 错误处理测试

use mquickjs_rs::async_error::AsyncError;
use mquickjs_rs::context::Context;
use mquickjs_rs::handles::local::{Function, Local, Value};
use mquickjs_rs::Root;

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
    // 验证 AsyncError 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<AsyncError>();
}

#[test]
fn test_async_error_static() {
    // 验证 AsyncError 是 'static
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

#[test]
fn test_async_error_to_js() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let error = AsyncError::Message("test error".to_string());
    let js_error = error.to_js(&scope);

    // 验证返回的是有效的 JS 值
    // 注意：这里我们无法直接检查是否是 Error 对象
    // 但可以验证不会 panic
    let _raw = js_error.as_raw();
}

#[test]
fn test_async_error_from_js() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建一个 JS Error
    let js_error = ctx.eval_jsvalue("new Error('test error')").unwrap();
    let error = AsyncError::from_js(&scope, scope.value(js_error));

    // 验证错误消息
    assert_eq!(error.message(), "test error");
}

#[test]
fn test_async_error_from_js_type_error() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建一个 JS TypeError
    let js_error = ctx.eval_jsvalue("new TypeError('type error')").unwrap();
    let error = AsyncError::from_js(&scope, scope.value(js_error));

    // 验证错误消息
    assert_eq!(error.message(), "type error");
}

#[test]
fn test_async_error_roundtrip() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建 Rust 错误
    let original = AsyncError::Message("test error".to_string());

    // 转换为 JS 错误
    let js_error = original.to_js(&scope);

    // 转换回 Rust 错误
    let restored = AsyncError::from_js(&scope, js_error);

    // 验证消息一致
    assert_eq!(original.message(), restored.message());
}