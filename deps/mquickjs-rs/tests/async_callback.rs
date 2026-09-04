//! Async callback integration tests

use mquickjs_rs::async_bridge::AsyncBridge;
use mquickjs_rs::async_task::{AsyncTaskManager, CompletionItem, TaskPriority};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_completion_queue_push_drain() {
    let manager = AsyncTaskManager::new();

    // Push completions
    manager.push_completion(CompletionItem {
        task_id: 1,
        result: Ok("result1".to_string()),
    });
    manager.push_completion(CompletionItem {
        task_id: 2,
        result: Err("error2".to_string()),
    });

    assert!(manager.has_completions());

    // Drain completions
    let items = manager.drain_completions();
    assert_eq!(items.len(), 2);
    assert!(!manager.has_completions());

    // Verify order (FIFO)
    assert_eq!(items[0].task_id, 1);
    assert_eq!(items[0].result, Ok("result1".to_string()));
    assert_eq!(items[1].task_id, 2);
    assert_eq!(items[1].result, Err("error2".to_string()));
}

#[test]
fn test_callback_registry() {
    let manager = AsyncTaskManager::new();

    // Register callback
    manager.register_callback(1, 12345);
    manager.register_callback(2, 67890);

    // Take callback
    let cb1 = manager.take_callback(1);
    assert_eq!(cb1, Some(12345));

    // Callback should be removed after take
    let cb1_again = manager.take_callback(1);
    assert_eq!(cb1_again, None);

    // Other callbacks should still be there
    let cb2 = manager.take_callback(2);
    assert_eq!(cb2, Some(67890));
}

#[test]
fn test_spawn_cancellable_with_queue() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let task_id = task_manager.register_task(TaskPriority::Cancellable);

    // Spawn task that returns a string
    bridge.spawn_cancellable_with_queue(async {
        "hello".to_string()
    }, task_id);

    // Wait for task to complete
    std::thread::sleep(Duration::from_millis(100));

    // Check completion queue
    assert!(task_manager.has_completions());
    let items = task_manager.drain_completions();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].task_id, task_id);
    assert_eq!(items[0].result, Ok("hello".to_string()));
}

#[test]
fn test_spawn_non_cancellable_with_queue() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let task_id = task_manager.register_task(TaskPriority::NonCancellable);

    bridge.spawn_non_cancellable_with_queue(async {
        "world".to_string()
    }, task_id);

    std::thread::sleep(Duration::from_millis(100));

    assert!(task_manager.has_completions());
    let items = task_manager.drain_completions();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].result, Ok("world".to_string()));
}

#[test]
fn test_spawn_with_timeout_with_queue() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let task_id = task_manager.register_task(TaskPriority::Timeout(5000));

    bridge.spawn_with_timeout_with_queue(async {
        "timeout_result".to_string()
    }, task_id, 5000);

    std::thread::sleep(Duration::from_millis(100));

    assert!(task_manager.has_completions());
    let items = task_manager.drain_completions();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].result, Ok("timeout_result".to_string()));
}

#[test]
fn test_multiple_tasks_with_queue() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    // Spawn multiple tasks
    for i in 0..5 {
        let task_id = task_manager.register_task(TaskPriority::Cancellable);
        bridge.spawn_cancellable_with_queue(async move {
            format!("result_{}", i)
        }, task_id);
    }

    // Wait for all tasks to complete
    std::thread::sleep(Duration::from_millis(200));

    // Drain all completions
    let items = task_manager.drain_completions();
    assert_eq!(items.len(), 5);

    // Verify all results exist (order may vary due to parallel execution)
    let mut results: Vec<String> = items.iter().map(|i| i.result.clone().unwrap()).collect();
    results.sort();
    for i in 0..5 {
        assert_eq!(results[i], format!("result_{}", i));
    }
}

#[test]
fn test_callback_with_completion() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let task_id = task_manager.register_task(TaskPriority::Cancellable);

    // Register a mock callback (just a number for testing)
    task_manager.register_callback(task_id, 99999);

    // Spawn task
    bridge.spawn_cancellable_with_queue(async {
        "callback_result".to_string()
    }, task_id);

    std::thread::sleep(Duration::from_millis(100));

    // Take callback
    let callback = task_manager.take_callback(task_id);
    assert_eq!(callback, Some(99999));

    // Drain completion
    let items = task_manager.drain_completions();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].result, Ok("callback_result".to_string()));
}

#[test]
fn test_empty_drain() {
    let task_manager = AsyncTaskManager::new();

    // Drain empty queue
    let items = task_manager.drain_completions();
    assert!(items.is_empty());
    assert!(!task_manager.has_completions());
}