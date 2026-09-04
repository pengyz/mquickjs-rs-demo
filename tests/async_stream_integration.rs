//! AsyncStream 集成测试
//!
//! 演示如何使用 AsyncStream 进行异步回调

#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::async_error::AsyncError;
#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::async_stream::{AsyncStream, EventCompletion, ThreadSafeEventQueue};
#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::context::Context;
#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::handles::local::{Function, Local, Value};
#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::Root;
#[cfg(feature = "ridl-extensions")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "ridl-extensions")]
use std::thread;
#[cfg(feature = "ridl-extensions")]
use std::time::Duration;

/// 测试基本的 AsyncStream 使用
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_async_stream_basic_usage() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建事件流
    let mut stream = AsyncStream::<i32>::new();

    // 创建 callback
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    // 订阅事件
    let sub = unsafe { stream.subscribe(cb_root) };
    assert_eq!(stream.subscriber_count(), 1);

    // 发射事件
    unsafe {
        stream.emit(&scope, &42);
    }

    // 取消订阅
    sub.unsubscribe(&mut stream);
    assert_eq!(stream.subscriber_count(), 0);
}

/// 测试 AsyncStream 错误处理
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_async_stream_error_handling() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建事件流
    let mut stream = AsyncStream::<i32>::new();

    // 创建 callback
    let callback = ctx.eval_jsvalue("(function(err, val) { return val; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    // 订阅事件
    let sub = unsafe { stream.subscribe(cb_root) };

    // 发射错误
    let error = AsyncError::Message("test error".to_string());
    unsafe {
        stream.emit_error(&scope, &error);
    }

    // 取消订阅
    sub.unsubscribe(&mut stream);
}

/// 测试线程安全的事件队列
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_thread_safe_event_queue() {
    let queue = Arc::new(ThreadSafeEventQueue::<String>::new());
    let mut handles = vec![];

    // 启动多个线程推送事件
    for i in 0..4 {
        let queue_clone = queue.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let event = EventCompletion {
                    stream_id: i,
                    value: format!("event_{}_{}", i, j),
                };
                queue_clone.push(event);
            }
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证所有事件都被推送
    assert_eq!(queue.len(), 40);

    // 批量弹出所有事件
    let events = queue.drain();
    assert_eq!(events.len(), 40);
    assert!(queue.is_empty());
}

/// 测试 AsyncStream 生命周期管理
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_async_stream_lifecycle() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建事件流
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

/// 测试 AsyncStream 多个 subscriber
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_async_stream_multiple_subscribers() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建事件流
    let mut stream = AsyncStream::<i32>::new();

    // 创建多个 subscription
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

    // 发射事件
    unsafe {
        stream.emit(&scope, &42);
    }

    // 清空 stream
    stream.clear();
    assert_eq!(stream.subscriber_count(), 0);
}

/// 测试 AsyncStream 与线程安全队列的集成
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_async_stream_with_thread_safe_queue() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 创建事件流
    let mut stream = AsyncStream::<String>::new();

    // 创建 callback
    let callback = ctx.eval_jsvalue("(function(v) { return v; })").unwrap();
    let callback_local = scope.value(callback);
    let function_local = callback_local.try_into_function(&scope).unwrap();
    let cb_root = Root::new(&scope, function_local);

    // 订阅事件
    let sub = unsafe { stream.subscribe(cb_root) };

    // 创建线程安全队列
    let queue = Arc::new(ThreadSafeEventQueue::<String>::new());

    // 在另一个线程中推送事件
    let queue_clone = queue.clone();
    let handle = thread::spawn(move || {
        for i in 0..5 {
            let event = EventCompletion {
                stream_id: 1,
                value: format!("event_{}", i),
            };
            queue_clone.push(event);
        }
    });

    // 等待线程完成
    handle.join().unwrap();

    // 验证队列中有事件
    assert_eq!(queue.len(), 5);

    // 批量弹出事件
    let events = queue.drain();
    assert_eq!(events.len(), 5);

    // 取消订阅
    sub.unsubscribe(&mut stream);
}