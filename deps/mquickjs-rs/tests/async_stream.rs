//! AsyncStream 测试

use mquickjs_rs::async_stream::{AsyncStream, Subscription};
use mquickjs_rs::context::Context;
use mquickjs_rs::handles::local::{Local, Value};
use mquickjs_rs::Root;

#[test]
fn test_async_stream_new() {
    let stream = AsyncStream::<i32>::new();
    assert_eq!(stream.subscriber_count(), 0);
}

#[test]
fn test_async_stream_subscribe() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建一个简单的 callback
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    unsafe {
        let sub = stream.subscribe(cb_root);
        assert_eq!(stream.subscriber_count(), 1);
        assert!(sub.id() > 0);
    }
}

#[test]
fn test_async_stream_unsubscribe() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建一个简单的 callback
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    unsafe {
        let sub = stream.subscribe(cb_root);
        assert_eq!(stream.subscriber_count(), 1);

        sub.unsubscribe(&mut stream);
        assert_eq!(stream.subscriber_count(), 0);
    }
}

#[test]
fn test_async_stream_multiple_subscribers() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 callback
    let mut subscriptions = Vec::new();
    for _ in 0..3 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            subscriptions.push(sub);
        }
    }

    assert_eq!(stream.subscriber_count(), 3);
}

#[test]
fn test_async_stream_clear() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 callback
    let mut subscriptions = Vec::new();
    for _ in 0..3 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            subscriptions.push(sub);
        }
    }

    assert_eq!(stream.subscriber_count(), 3);

    stream.clear();
    assert_eq!(stream.subscriber_count(), 0);
}

#[test]
fn test_subscription_drop() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建一个简单的 callback
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    unsafe {
        {
            let sub = stream.subscribe(cb_root);
            assert_eq!(stream.subscriber_count(), 1);
            // sub 在这里 drop
        }
        // subscriber 应该已经被移除
        assert_eq!(stream.subscriber_count(), 0);
    }
}

#[test]
fn test_async_stream_send() {
    // 验证 AsyncStream 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<AsyncStream<i32>>();
}

#[test]
fn test_async_stream_static() {
    // 验证 AsyncStream 是 'static
    fn assert_static<T: 'static>() {}
    assert_static::<AsyncStream<i32>>();
}

#[test]
fn test_subscription_send() {
    // 验证 Subscription 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<Subscription>();
}

#[test]
fn test_subscription_static() {
    // 验证 Subscription 是 'static
    fn assert_static<T: 'static>() {}
    assert_static::<Subscription>();
}

#[test]
fn test_async_stream_emit_calls_callback() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建一个会记录调用的 callback
    let callback = ctx.eval_jsvalue(
        r#"
        (function(v) {
            globalThis.__async_stream_test_result = v;
            return v;
        })
        "#,
    ).unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    // 订阅事件
    let sub = unsafe { stream.subscribe(cb_root) };

    // 发射事件
    unsafe {
        stream.emit(&scope, &42);
    }

    // 验证 callback 被调用
    let result = ctx.eval_jsvalue("globalThis.__async_stream_test_result").unwrap();
    
    // 检查结果是否为 42
    let mut num: i32 = 0;
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_ToInt32(scope.ctx(), &mut num as *mut i32, result);
    }
    assert_eq!(num, 42);

    // 清理
    sub.unsubscribe(&mut stream);
}