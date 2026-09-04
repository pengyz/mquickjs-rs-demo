//! AsyncStream 线程安全测试

use mquickjs_rs::async_stream::{AsyncStream, EventCompletion, EventQueue, ThreadSafeEventQueue};
use mquickjs_rs::context::Context;
use mquickjs_rs::handles::local::{Function, Local, Value};
use mquickjs_rs::Root;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_event_queue_new() {
    let queue = EventQueue::<i32>::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_event_queue_push_pop() {
    let mut queue = EventQueue::<i32>::new();

    // 推送事件
    queue.push(EventCompletion {
        stream_id: 1,
        value: 42,
    });

    assert!(!queue.is_empty());
    assert_eq!(queue.len(), 1);

    // 弹出事件
    let event = queue.pop();
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.stream_id, 1);
    assert_eq!(event.value, 42);

    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_event_queue_multiple_events() {
    let mut queue = EventQueue::<i32>::new();

    // 推送多个事件
    for i in 0..5 {
        queue.push(EventCompletion {
            stream_id: i as u64,
            value: i * 10,
        });
    }

    assert_eq!(queue.len(), 5);

    // 弹出所有事件（FIFO 顺序）
    for i in 0..5 {
        let event = queue.pop().unwrap();
        assert_eq!(event.stream_id, i as u64);
        assert_eq!(event.value, i * 10);
    }

    assert!(queue.is_empty());
}

#[test]
fn test_event_queue_thread_safety() {
    let queue = Arc::new(Mutex::new(EventQueue::<i32>::new()));
    let mut handles = vec![];

    // 启动多个线程推送事件
    for i in 0..4 {
        let queue_clone = queue.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let event = EventCompletion {
                    stream_id: (i * 10 + j) as u64,
                    value: i * 100 + j,
                };
                queue_clone.lock().unwrap().push(event);
            }
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证所有事件都被推送
    let queue = queue.lock().unwrap();
    assert_eq!(queue.len(), 40);
}

#[test]
fn test_event_queue_pop_empty() {
    let mut queue = EventQueue::<i32>::new();
    assert!(queue.pop().is_none());
}

#[test]
fn test_event_queue_clear() {
    let mut queue = EventQueue::<i32>::new();

    // 推送事件
    for i in 0..5 {
        queue.push(EventCompletion {
            stream_id: i as u64,
            value: i,
        });
    }

    assert_eq!(queue.len(), 5);

    // 清空队列
    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_event_queue_send() {
    // 验证 EventQueue 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<EventQueue<i32>>();
}

#[test]
fn test_event_queue_sync() {
    // 验证 EventQueue 是 Sync（通过 Arc<Mutex<EventQueue>>）
    fn assert_sync<T: Sync>() {}
    assert_sync::<Arc<Mutex<EventQueue<i32>>>>();
}

#[test]
fn test_event_completion_send() {
    // 验证 EventCompletion 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<EventCompletion<i32>>();
}

#[test]
fn test_event_completion_clone() {
    let event = EventCompletion {
        stream_id: 1,
        value: 42,
    };
    let cloned = event.clone();
    assert_eq!(event.stream_id, cloned.stream_id);
    assert_eq!(event.value, cloned.value);
}

#[test]
fn test_event_queue_drain() {
    let mut queue = EventQueue::<i32>::new();

    // 推送事件
    for i in 0..5 {
        queue.push(EventCompletion {
            stream_id: i as u64,
            value: i * 10,
        });
    }

    assert_eq!(queue.len(), 5);

    // 批量弹出所有事件
    let events = queue.drain();
    assert_eq!(events.len(), 5);
    assert!(queue.is_empty());

    // 验证事件顺序
    for (i, event) in events.iter().enumerate() {
        assert_eq!(event.stream_id, i as u64);
        assert_eq!(event.value, i as i32 * 10);
    }
}

#[test]
fn test_event_queue_drain_empty() {
    let mut queue = EventQueue::<i32>::new();
    let events = queue.drain();
    assert!(events.is_empty());
}

#[test]
fn test_thread_safe_event_queue_new() {
    let queue = ThreadSafeEventQueue::<i32>::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_thread_safe_event_queue_push_drain() {
    let queue = ThreadSafeEventQueue::<i32>::new();

    // 推送事件
    for i in 0..5 {
        queue.push(EventCompletion {
            stream_id: i as u64,
            value: i * 10,
        });
    }

    assert_eq!(queue.len(), 5);

    // 批量弹出所有事件
    let events = queue.drain();
    assert_eq!(events.len(), 5);
    assert!(queue.is_empty());

    // 验证事件顺序
    for (i, event) in events.iter().enumerate() {
        assert_eq!(event.stream_id, i as u64);
        assert_eq!(event.value, i as i32 * 10);
    }
}

#[test]
fn test_thread_safe_event_queue_thread_safety() {
    let queue = Arc::new(ThreadSafeEventQueue::<i32>::new());
    let mut handles = vec![];

    // 启动多个线程推送事件
    for i in 0..4 {
        let queue_clone = queue.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let event = EventCompletion {
                    stream_id: (i * 10 + j) as u64,
                    value: i * 100 + j,
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

#[test]
fn test_thread_safe_event_queue_clear() {
    let queue = ThreadSafeEventQueue::<i32>::new();

    // 推送事件
    for i in 0..5 {
        queue.push(EventCompletion {
            stream_id: i as u64,
            value: i,
        });
    }

    assert_eq!(queue.len(), 5);

    // 清空队列
    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn test_thread_safe_event_queue_send() {
    // 验证 ThreadSafeEventQueue 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<ThreadSafeEventQueue<i32>>();
}

#[test]
fn test_thread_safe_event_queue_sync() {
    // 验证 ThreadSafeEventQueue 是 Sync
    fn assert_sync<T: Sync>() {}
    assert_sync::<ThreadSafeEventQueue<i32>>();
}