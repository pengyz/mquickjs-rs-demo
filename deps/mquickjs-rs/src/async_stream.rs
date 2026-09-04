//! AsyncStream: 事件流机制
//!
//! 核心设计：
//! - AsyncStream<T> 管理多个 subscriber
//! - Subscription 管理单个 subscriber 的生命周期
//! - emit() 只能在 JS 主线程调用
//! - unsubscribe() 可以在任意线程调用

use crate::handles::local::{Function, Local, Value};
use crate::handles::scope::Scope;
use crate::Root;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, Weak};

/// 订阅句柄 - 管理 callback 生命周期
///
/// # Safety
/// - Drop 时自动从 AsyncStream 中移除 subscriber
/// - 可以在任意线程调用 unsubscribe()
pub struct Subscription {
    id: u64,
    // 使用 Weak 避免循环引用
    stream_weak: Weak<Mutex<AsyncStreamInner>>,
}

impl Subscription {
    /// 创建新的 Subscription
    ///
    /// # Safety
    /// - stream_weak 必须指向有效的 AsyncStream
    pub(crate) fn new(
        id: u64,
        stream_weak: Weak<Mutex<AsyncStreamInner>>,
    ) -> Self {
        Self {
            id,
            stream_weak,
        }
    }

    /// 获取订阅 ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 取消订阅
    ///
    /// # Safety
    /// - 可以在任意线程调用
    /// - 调用后 Subscription 失效
    pub fn unsubscribe(self, stream: &mut AsyncStream<impl Send + 'static>) {
        stream.remove(self.id);
        // 防止 Drop 再次调用 remove
        std::mem::forget(self);
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // 从 AsyncStream 中移除 subscriber
        if let Some(stream) = self.stream_weak.upgrade() {
            let mut inner = stream.lock().unwrap();
            inner.subscribers.retain(|entry| entry.id != self.id);
        }
    }
}

// Safety: Subscription 可以安全地跨线程发送
// 因为它只包含 id 和 Weak，不持有 JS 状态
unsafe impl Send for Subscription {}

/// Subscriber 条目
struct SubscriberEntry<T> {
    id: u64,
    callback: Root<Function>,
    _marker: PhantomData<T>,
}

// Safety: SubscriberEntry 可以安全地跨线程发送
// 因为 Root<Function> 是 Send
unsafe impl<T> Send for SubscriberEntry<T> {}

/// AsyncStream 内部状态
struct AsyncStreamInner {
    subscribers: Vec<SubscriberEntry<i32>>,  // 使用 i32 作为占位符
    next_id: u64,
}

/// 事件发射器
///
/// # Safety
/// - subscribe() 和 emit() 只能在 JS 主线程调用
/// - unsubscribe() 和 clear() 可以在任意线程调用
pub struct AsyncStream<T: Send + 'static> {
    inner: Arc<Mutex<AsyncStreamInner>>,
    _marker: PhantomData<T>,
}

// Safety: AsyncStream 可以安全地跨线程发送
// 因为它只包含 Arc<Mutex<...>>，不持有 JS 状态
unsafe impl<T: Send + 'static> Send for AsyncStream<T> {}

impl<T: Send + 'static> AsyncStream<T> {
    /// 创建新的 AsyncStream
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AsyncStreamInner {
                subscribers: Vec::new(),
                next_id: 1,
            })),
            _marker: PhantomData,
        }
    }

    /// 获取 subscriber 数量
    pub fn subscriber_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.subscribers.len()
    }

    /// 订阅事件
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    /// - callback 必须是有效的 JS 函数
    pub unsafe fn subscribe(&mut self, callback: Root<Function>) -> Subscription {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id;
        inner.next_id += 1;

        inner.subscribers.push(SubscriberEntry {
            id,
            callback,
            _marker: PhantomData,
        });

        // 创建 Subscription，传入 Weak 引用
        let stream_weak = Arc::downgrade(&self.inner);
        Subscription::new(id, stream_weak)
    }

    /// 移除指定 subscriber
    fn remove(&mut self, id: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.retain(|entry| entry.id != id);
    }

    /// 发射事件
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    /// - value 必须是有效的 JS 值
    pub unsafe fn emit(&self, scope: &Scope<'_>, value: &T)
    where
        T: ToJsValue,
    {
        let inner = self.inner.lock().unwrap();
        for entry in &inner.subscribers {
            let js_value = value.to_js_value(scope);
            self.call_callback(scope, &entry.callback, js_value);
        }
    }

    /// 调用 callback
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    unsafe fn call_callback(
        &self,
        scope: &Scope<'_>,
        callback: &Root<Function>,
        value: Local<'_, Value>,
    ) {
        // 将 Root<Function> 转换为 Local<Function>
        let callback_local = callback.to_local(scope);
        
        // 调用 callback
        let this_val = scope.value(crate::mquickjs_ffi::JS_UNDEFINED);
        let args = [value];
        
        match callback_local.call(scope, this_val, &args) {
            Ok(_) => {},
            Err(e) => {
                // TODO: 错误处理
                eprintln!("AsyncStream callback error: {}", e);
            }
        }
    }

    /// 清空所有 subscriber
    pub fn clear(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.clear();
    }

    /// 处理事件队列中的所有事件
    ///
    /// # Safety
    /// - 只能在 JS 主线程调用
    /// - 事件队列必须是有效的
    pub unsafe fn drain_events(
        &self,
        scope: &Scope<'_>,
        queue: &mut EventQueue<T>,
    ) where
        T: ToJsValue,
    {
        let events = queue.drain();
        let inner = self.inner.lock().unwrap();
        for event in events {
            // 找到对应的 stream
            // 注意：这里假设所有事件都属于当前 stream
            // 实际实现中需要根据 stream_id 路由到正确的 stream
            let js_value = event.value.to_js_value(scope);
            for entry in &inner.subscribers {
                // 复制 js_value，因为 call_callback 会消耗它
                let js_value_copy = scope.value(js_value.as_raw());
                self.call_callback(scope, &entry.callback, js_value_copy);
            }
        }
    }
}

impl<T: Send + 'static> Drop for AsyncStream<T> {
    fn drop(&mut self) {
        // 清空所有 subscriber
        self.clear();
    }
}

/// 将 Rust 值转换为 JS 值的 trait
pub trait ToJsValue {
    /// 将 Rust 值转换为 JS 值
    ///
    /// # Safety
    /// - 必须在 JS 主线程调用
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value>;
}

/// 事件完成项 - 用于 Worker 线程向 JS 主线程传递事件
///
/// # Safety
/// - T 必须是 Send + 'static
/// - 可以安全地跨线程发送
#[derive(Debug, Clone)]
pub struct EventCompletion<T: Send + 'static> {
    /// 事件流 ID
    pub stream_id: u64,
    /// 事件值
    pub value: T,
}

// Safety: EventCompletion 可以安全地跨线程发送
// 因为 T 是 Send + 'static
unsafe impl<T: Send + 'static> Send for EventCompletion<T> {}

/// 事件队列 - 用于 Worker 线程向 JS 主线程传递事件
///
/// # Safety
/// - 可以在任意线程调用 push()
/// - 只能在 JS 主线程调用 pop() 和 drain()
pub struct EventQueue<T: Send + 'static> {
    queue: Vec<EventCompletion<T>>,
}

// Safety: EventQueue 可以安全地跨线程发送
// 因为它只包含 Vec，不持有 JS 状态
unsafe impl<T: Send + 'static> Send for EventQueue<T> {}

impl<T: Send + 'static> EventQueue<T> {
    /// 创建新的事件队列
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
        }
    }

    /// 推送事件到队列
    ///
    /// # Safety
    /// - 可以在任意线程调用
    pub fn push(&mut self, event: EventCompletion<T>) {
        self.queue.push(event);
    }

    /// 从队列弹出事件
    ///
    /// # Safety
    /// - 只能在 JS 主线程调用
    pub fn pop(&mut self) -> Option<EventCompletion<T>> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 获取队列长度
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// 批量弹出所有事件
    ///
    /// # Safety
    /// - 只能在 JS 主线程调用
    pub fn drain(&mut self) -> Vec<EventCompletion<T>> {
        let events = self.queue.drain(..).collect();
        events
    }
}

/// 线程安全的事件队列包装器
///
/// 用于在多个线程之间共享事件队列
///
/// # Safety
/// - 可以在任意线程调用 push()
/// - 只能在 JS 主线程调用 drain()
pub struct ThreadSafeEventQueue<T: Send + 'static> {
    inner: std::sync::Mutex<EventQueue<T>>,
}

// Safety: ThreadSafeEventQueue 可以安全地跨线程发送和共享
// 因为它使用 Mutex 保护内部状态
unsafe impl<T: Send + 'static> Send for ThreadSafeEventQueue<T> {}
unsafe impl<T: Send + 'static> Sync for ThreadSafeEventQueue<T> {}

impl<T: Send + 'static> ThreadSafeEventQueue<T> {
    /// 创建新的线程安全事件队列
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(EventQueue::new()),
        }
    }

    /// 推送事件到队列（线程安全）
    ///
    /// # Safety
    /// - 可以在任意线程调用
    pub fn push(&self, event: EventCompletion<T>) {
        let mut queue = self.inner.lock().unwrap();
        queue.push(event);
    }

    /// 批量弹出所有事件（只能在 JS 主线程调用）
    ///
    /// # Safety
    /// - 只能在 JS 主线程调用
    pub fn drain(&self) -> Vec<EventCompletion<T>> {
        let mut queue = self.inner.lock().unwrap();
        queue.drain()
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        let queue = self.inner.lock().unwrap();
        queue.is_empty()
    }

    /// 获取队列长度
    pub fn len(&self) -> usize {
        let queue = self.inner.lock().unwrap();
        queue.len()
    }

    /// 清空队列
    pub fn clear(&self) {
        let mut queue = self.inner.lock().unwrap();
        queue.clear();
    }
}

// 为基本类型实现 ToJsValue
impl ToJsValue for i32 {
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value> {
        // TODO: 实现 i32 到 JSValue 的转换
        todo!()
    }
}

impl ToJsValue for f64 {
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value> {
        // TODO: 实现 f64 到 JSValue 的转换
        todo!()
    }
}

impl ToJsValue for bool {
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value> {
        // TODO: 实现 bool 到 JSValue 的转换
        todo!()
    }
}

impl ToJsValue for String {
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value> {
        // TODO: 实现 String 到 JSValue 的转换
        todo!()
    }
}

impl ToJsValue for &str {
    unsafe fn to_js_value(&self, scope: &Scope<'_>) -> Local<'_, Value> {
        // TODO: 实现 &str 到 JSValue 的转换
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_stream_new() {
        let stream = AsyncStream::<i32>::new();
        assert_eq!(stream.subscriber_count(), 0);
    }

    #[test]
    fn test_async_stream_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AsyncStream<i32>>();
    }

    #[test]
    fn test_async_stream_static() {
        fn assert_static<T: 'static>() {}
        assert_static::<AsyncStream<i32>>();
    }

    #[test]
    fn test_subscription_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Subscription>();
    }

    #[test]
    fn test_subscription_static() {
        fn assert_static<T: 'static>() {}
        assert_static::<Subscription>();
    }
}