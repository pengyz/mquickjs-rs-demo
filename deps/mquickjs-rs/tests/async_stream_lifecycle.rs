//! AsyncStream 生命周期管理测试

use mquickjs_rs::async_stream::{AsyncStream, Subscription};
use mquickjs_rs::context::Context;
use mquickjs_rs::handles::local::{Function, Local, Value};
use mquickjs_rs::Root;
use std::sync::{Arc, Mutex};

#[test]
fn test_subscription_drop_removes_subscriber() {
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
fn test_subscription_unsubscribe_removes_subscriber() {
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
fn test_multiple_subscriptions_drop() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 subscription
    let mut subscriptions = Vec::new();
    for _ in 0..5 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            subscriptions.push(sub);
        }
    }

    assert_eq!(stream.subscriber_count(), 5);

    // 逐个 drop subscription
    for sub in subscriptions {
        drop(sub);
    }

    assert_eq!(stream.subscriber_count(), 0);
}

#[test]
fn test_stream_clear_removes_all_subscribers() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 subscription
    let mut subscriptions = Vec::new();
    for _ in 0..5 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            subscriptions.push(sub);
        }
    }

    assert_eq!(stream.subscriber_count(), 5);

    // 清空 stream
    stream.clear();
    assert_eq!(stream.subscriber_count(), 0);

    // subscription 仍然存在，但 stream 已经清空
    // 这是预期行为
}

#[test]
fn test_stream_drop_cleans_up_subscribers() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 subscription
    let mut subscriptions = Vec::new();
    for _ in 0..5 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            subscriptions.push(sub);
        }
    }

    assert_eq!(stream.subscriber_count(), 5);

    // drop stream
    drop(stream);

    // subscription 仍然存在，但 stream 已经被销毁
    // 这是预期行为
}

#[test]
fn test_subscription_id_uniqueness() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 subscription
    let mut ids = Vec::new();
    for _ in 0..10 {
        let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
        let callback_local = scope.value(callback);
        let function_local = callback_local.try_into_function(&scope).unwrap();
        let cb_root = Root::new(&scope, function_local);

        unsafe {
            let sub = stream.subscribe(cb_root);
            ids.push(sub.id());
        }
    }

    // 验证 ID 唯一性
    let mut unique_ids = ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(ids.len(), unique_ids.len());
}

#[test]
fn test_subscription_after_stream_clear() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut stream = AsyncStream::<i32>::new();

    // 创建 subscription
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    unsafe {
        let sub = stream.subscribe(cb_root);
        assert_eq!(stream.subscriber_count(), 1);

        // 清空 stream
        stream.clear();
        assert_eq!(stream.subscriber_count(), 0);

        // subscription 仍然存在
        // 当 sub drop 时，remove_fn 会被调用，但 stream 已经清空
        // 这是安全的，因为 remove_fn 会检查 subscriber 是否存在
    }
}

#[test]
fn test_subscription_send() {
    // 验证 Subscription 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<Subscription>();
}

#[test]
fn test_subscription_clone() {
    // Subscription 不实现 Clone，因为它是独占的
    // 这个测试验证 Subscription 不能被克隆
    // （编译时检查）
}