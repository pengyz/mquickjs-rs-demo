//! TDD tests for async cancellation semantics
//!
//! Tests cover:
//! 1. Cancellable tasks (cancelled on context drop)
//! 2. Non-cancellable tasks (must complete even if context drops)
//! 3. Timeout tasks (cancelled after timeout)
//! 4. Root lifecycle (released when task completes)
//! 5. Graceful shutdown (process by priority)
//! 6. Error handling (panic, timeout, cancellation)

use mquickjs_rs::async_bridge::{AsyncBridge, AsyncBridgeError, AsyncCallback};
use mquickjs_rs::async_task::{AsyncTaskManager, TaskPriority, TaskStatus};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// A future that completes after a given duration, yielding on each poll
/// so that cancellation checks can run between polls.
struct DelayFuture {
    deadline: Instant,
}

impl DelayFuture {
    fn new(duration: Duration) -> Self {
        Self {
            deadline: Instant::now() + duration,
        }
    }
}

impl Future for DelayFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            // Yield to allow cancellation checks
            std::thread::yield_now();
            Poll::Pending
        }
    }
}

/// Test 1: Cancellable tasks are cancelled when cancelled
#[test]
fn test_cancellable_task_cancelled_on_context_drop() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a cancellable task with a long delay
    let task_id = bridge.spawn_cancellable(
        async {
            // Use a non-blocking delay future
            DelayFuture::new(Duration::from_secs(10)).await;
            "result".to_string()
        },
        Box::new(move |_result| {
            // This should not be called if task is cancelled
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    // Verify task is registered
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Pending)
    );

    // Wait a bit for the task to start
    std::thread::sleep(Duration::from_millis(50));

    // Cancel the task (simulating context drop)
    assert!(bridge.cancel_task(task_id));

    // Wait for cancellation to be processed
    std::thread::sleep(Duration::from_millis(200));

    // Verify task is cancelled
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Cancelled)
    );

    // Verify callback was not called
    assert!(!callback_called.load(Ordering::SeqCst));
}

/// Test 2: Non-cancellable tasks complete even if cancelled
#[test]
fn test_non_cancellable_task_completes_despite_cancellation() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a non-cancellable task
    let task_id = bridge.spawn_non_cancellable(
        async {
            // Quick non-blocking task
            DelayFuture::new(Duration::from_millis(50)).await;
            "non-cancellable result".to_string()
        },
        Box::new(move |result| {
            assert_eq!(result.unwrap(), "non-cancellable result");
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    // Verify task is registered
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Pending)
    );

    // Try to cancel the task (should fail for non-cancellable)
    assert!(!bridge.cancel_task(task_id));

    // Wait for task to complete
    std::thread::sleep(Duration::from_millis(200));

    // Verify task completed
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Completed)
    );

    // Verify callback was called
    assert!(callback_called.load(Ordering::SeqCst));
}

/// Test 3: Timeout tasks are cancelled after timeout
#[test]
fn test_timeout_task_cancelled_after_timeout() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a task with a short timeout
    let task_id = bridge.spawn_with_timeout(
        async {
            // Use a long delay that will exceed the timeout
            DelayFuture::new(Duration::from_secs(10)).await;
            "result".to_string()
        },
        Box::new(move |_result| {
            // This should not be called if task times out
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
        100, // 100ms timeout
    );

    // Verify task is registered
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Pending)
    );

    // Wait for timeout to occur
    std::thread::sleep(Duration::from_millis(300));

    // Verify task is cancelled due to timeout
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Cancelled)
    );

    // Verify callback was not called
    assert!(!callback_called.load(Ordering::SeqCst));
}

/// Test 4: Root lifecycle - task completes and releases resources
#[test]
fn test_root_lifecycle_task_completes_and_releases() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a task that completes quickly
    let task_id = bridge.spawn_cancellable(
        async {
            DelayFuture::new(Duration::from_millis(50)).await;
            "quick result".to_string()
        },
        Box::new(move |result| {
            assert_eq!(result.unwrap(), "quick result");
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    // Verify task is registered
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Pending)
    );

    // Wait for task to complete
    std::thread::sleep(Duration::from_millis(200));

    // Verify task completed
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Completed)
    );

    // Verify callback was called
    assert!(callback_called.load(Ordering::SeqCst));

    // Verify task count is correct
    assert_eq!(task_manager.active_task_count(), 0);
}

/// Test 5: Graceful shutdown - process tasks by priority
#[test]
fn test_graceful_shutdown_by_priority() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let cancellable_called = Arc::new(AtomicBool::new(false));
    let non_cancellable_called = Arc::new(AtomicBool::new(false));
    let timeout_called = Arc::new(AtomicBool::new(false));

    let cancellable_called_clone = cancellable_called.clone();
    let non_cancellable_called_clone = non_cancellable_called.clone();
    let timeout_called_clone = timeout_called.clone();

    // Spawn tasks with different priorities
    let cancellable_id = bridge.spawn_cancellable(
        async {
            DelayFuture::new(Duration::from_millis(100)).await;
            "cancellable".to_string()
        },
        Box::new(move |result| {
            assert_eq!(result.unwrap(), "cancellable");
            cancellable_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    let non_cancellable_id = bridge.spawn_non_cancellable(
        async {
            DelayFuture::new(Duration::from_millis(100)).await;
            "non-cancellable".to_string()
        },
        Box::new(move |result| {
            assert_eq!(result.unwrap(), "non-cancellable");
            non_cancellable_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    let timeout_id = bridge.spawn_with_timeout(
        async {
            DelayFuture::new(Duration::from_millis(100)).await;
            "timeout".to_string()
        },
        Box::new(move |result| {
            assert_eq!(result.unwrap(), "timeout");
            timeout_called_clone.store(true, Ordering::SeqCst);
        }),
        5000, // 5 second timeout
    );

    // Verify all tasks are registered
    assert_eq!(task_manager.active_task_count(), 3);

    // Cancel all cancellable tasks (simulating context drop)
    let cancelled_ids = task_manager.cancel_all_cancellable();
    assert_eq!(cancelled_ids.len(), 1);
    assert!(cancelled_ids.contains(&cancellable_id));

    // Wait for non-cancellable and timeout tasks to complete
    std::thread::sleep(Duration::from_millis(300));

    // Verify cancellable task was cancelled
    assert_eq!(
        task_manager.get_task_status(cancellable_id),
        Some(TaskStatus::Cancelled)
    );
    assert!(!cancellable_called.load(Ordering::SeqCst));

    // Verify non-cancellable task completed
    assert_eq!(
        task_manager.get_task_status(non_cancellable_id),
        Some(TaskStatus::Completed)
    );
    assert!(non_cancellable_called.load(Ordering::SeqCst));

    // Verify timeout task completed (within timeout)
    assert_eq!(
        task_manager.get_task_status(timeout_id),
        Some(TaskStatus::Completed)
    );
    assert!(timeout_called.load(Ordering::SeqCst));
}

/// Test 6: Error handling - cancellation error
#[test]
fn test_error_handling_cancellation() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a task that will be cancelled
    let task_id = bridge.spawn_cancellable(
        async {
            DelayFuture::new(Duration::from_secs(10)).await;
            "result".to_string()
        },
        Box::new(move |_result| {
            // This should NOT be called when task is cancelled
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
    );

    // Wait for task to start
    std::thread::sleep(Duration::from_millis(50));

    // Cancel the task
    assert!(bridge.cancel_task(task_id));

    // Wait for cancellation to be processed
    std::thread::sleep(Duration::from_millis(200));

    // Verify task is cancelled
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Cancelled)
    );

    // Verify callback was NOT called
    assert!(!callback_called.load(Ordering::SeqCst));
}

/// Test 7: Error handling - timeout error
#[test]
fn test_error_handling_timeout() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // Spawn a task that will timeout
    let task_id = bridge.spawn_with_timeout(
        async {
            DelayFuture::new(Duration::from_secs(10)).await;
            "result".to_string()
        },
        Box::new(move |_result| {
            // This should NOT be called when task times out
            callback_called_clone.store(true, Ordering::SeqCst);
        }),
        100, // 100ms timeout
    );

    // Wait for timeout to occur
    std::thread::sleep(Duration::from_millis(300));

    // Verify task is cancelled due to timeout
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Cancelled)
    );

    // Verify callback was NOT called
    assert!(!callback_called.load(Ordering::SeqCst));
}

/// Test 8: Multiple tasks with different priorities
#[test]
fn test_multiple_tasks_different_priorities() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    let callback_count = Arc::new(AtomicU64::new(0));

    // Spawn multiple tasks with different priorities
    for i in 0..10 {
        let callback_count_clone = callback_count.clone();
        let priority = i % 3;

        match priority {
            0 => {
                bridge.spawn_cancellable(
                    async move {
                        DelayFuture::new(Duration::from_millis(50)).await;
                        format!("cancellable-{}", i)
                    },
                    Box::new(move |_result| {
                        callback_count_clone.fetch_add(1, Ordering::SeqCst);
                    }),
                );
            }
            1 => {
                bridge.spawn_non_cancellable(
                    async move {
                        DelayFuture::new(Duration::from_millis(50)).await;
                        format!("non-cancellable-{}", i)
                    },
                    Box::new(move |_result| {
                        callback_count_clone.fetch_add(1, Ordering::SeqCst);
                    }),
                );
            }
            _ => {
                bridge.spawn_with_timeout(
                    async move {
                        DelayFuture::new(Duration::from_millis(50)).await;
                        format!("timeout-{}", i)
                    },
                    Box::new(move |_result| {
                        callback_count_clone.fetch_add(1, Ordering::SeqCst);
                    }),
                    5000,
                );
            }
        };
    }

    // Verify all tasks are registered
    assert_eq!(task_manager.active_task_count(), 10);

    // Wait for all tasks to complete
    std::thread::sleep(Duration::from_millis(500));

    // Verify all callbacks were called
    assert_eq!(callback_count.load(Ordering::SeqCst), 10);
}

/// Test 9: Task cleanup after completion
#[test]
fn test_task_cleanup_after_completion() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = AsyncBridge::new(task_manager.clone());

    // Spawn and complete a task
    let task_id = bridge.spawn_cancellable(
        async {
            DelayFuture::new(Duration::from_millis(10)).await;
            "result".to_string()
        },
        Box::new(|_result| {}),
    );

    // Wait for task to complete
    std::thread::sleep(Duration::from_millis(100));

    // Verify task completed
    assert_eq!(
        task_manager.get_task_status(task_id),
        Some(TaskStatus::Completed)
    );

    // Cleanup finished tasks
    task_manager.cleanup_finished_tasks();

    // Verify task is removed
    assert_eq!(task_manager.get_task_status(task_id), None);
}

/// Test 10: Concurrent task spawning and cancellation
#[test]
fn test_concurrent_task_spawning_and_cancellation() {
    let task_manager = Arc::new(AsyncTaskManager::new());
    let bridge = Arc::new(AsyncBridge::new(task_manager.clone()));

    let mut handles = Vec::new();

    // Spawn tasks from multiple threads
    for i in 0..10 {
        let bridge_clone = bridge.clone();
        let handle = std::thread::spawn(move || {
            let task_id = bridge_clone.spawn_cancellable(
                async move {
                    DelayFuture::new(Duration::from_millis(50)).await;
                    format!("result-{}", i)
                },
                Box::new(|_result| {}),
            );
            task_id
        });
        handles.push(handle);
    }

    // Collect task IDs
    let mut task_ids = Vec::new();
    for handle in handles {
        task_ids.push(handle.join().unwrap());
    }

    // Verify all tasks are registered
    assert_eq!(task_manager.active_task_count(), 10);

    // Cancel all tasks
    for task_id in &task_ids {
        bridge.cancel_task(*task_id);
    }

    // Wait for cancellation to be processed
    std::thread::sleep(Duration::from_millis(200));

    // Verify all tasks are cancelled
    for task_id in task_ids {
        let status = task_manager.get_task_status(task_id);
        assert!(
            status == Some(TaskStatus::Cancelled) || status == Some(TaskStatus::Completed),
            "Expected Cancelled or Completed, got {:?}",
            status
        );
    }
}
