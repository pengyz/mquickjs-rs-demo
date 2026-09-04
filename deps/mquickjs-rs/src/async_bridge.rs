//! Async Bridge for RIDL Future→callback conversion
//!
//! This module provides utilities for bridging Rust Futures to JavaScript callbacks
//! with support for the RIDL async cancellation semantics.
//!
//! Design decisions from docs/knowledge/decision_ridl_async_cancellation.md:
//! - Default: cancellable (context drop cancels immediately)
//! - @nonCancellable: must complete even if context drops
//! - @timeout(ms): auto-cancel after timeout

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::async_task::{AsyncTaskManager, TaskPriority, TaskStatus};

/// Error type for async bridge operations
#[derive(Debug, Clone)]
pub enum AsyncBridgeError {
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
    /// Context was dropped
    ContextDropped,
    /// Future returned an error
    FutureError(String),
}

impl std::fmt::Display for AsyncBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncBridgeError::Cancelled => write!(f, "Task was cancelled"),
            AsyncBridgeError::TimedOut => write!(f, "Task timed out"),
            AsyncBridgeError::ContextDropped => write!(f, "Context was dropped"),
            AsyncBridgeError::FutureError(msg) => write!(f, "Future error: {}", msg),
        }
    }
}

impl std::error::Error for AsyncBridgeError {}

/// Result type for async bridge operations
pub type AsyncBridgeResult<T> = Result<T, AsyncBridgeError>;

/// Callback function type for async operations
pub type AsyncCallback<T> = Box<dyn FnOnce(AsyncBridgeResult<T>) + Send + 'static>;

/// Wrapper for a Future that can be cancelled
pub struct CancellableFuture<T> {
    /// The inner future
    future: Pin<Box<dyn Future<Output = T> + Send + 'static>>,
    /// Task ID in the AsyncTaskManager
    task_id: u64,
    /// Reference to the AsyncTaskManager
    task_manager: Arc<AsyncTaskManager>,
}

impl<T> CancellableFuture<T> {
    /// Create a new CancellableFuture
    pub fn new(
        future: impl Future<Output = T> + Send + 'static,
        task_id: u64,
        task_manager: Arc<AsyncTaskManager>,
    ) -> Self {
        Self {
            future: Box::pin(future),
            task_id,
            task_manager,
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Check if the future has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.task_manager
            .get_task_status(self.task_id)
            .map_or(false, |status| status == TaskStatus::Cancelled)
    }
}

impl<T> Future for CancellableFuture<T> {
    type Output = AsyncBridgeResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Check if the task has been cancelled
        if this.is_cancelled() {
            return Poll::Ready(Err(AsyncBridgeError::Cancelled));
        }

        // Check if the task has timed out
        if let Some(status) = this.task_manager.get_task_status(this.task_id) {
            if status == TaskStatus::TimedOut {
                return Poll::Ready(Err(AsyncBridgeError::TimedOut));
            }
        }

        // Check for timeout tasks
        if let Some(priority) = this.task_manager.get_task_priority(this.task_id) {
            if let TaskPriority::Timeout(_) = priority {
                // Check if the task has timed out
                let timed_out = this
                    .task_manager
                    .tasks
                    .lock()
                    .unwrap()
                    .get(&this.task_id)
                    .map_or(false, |task| task.should_cancel());

                if timed_out {
                    // Cancel the task
                    this.task_manager.cancel_task(this.task_id);
                    return Poll::Ready(Err(AsyncBridgeError::TimedOut));
                }
            }
        }

        // Poll the inner future
        match this.future.as_mut().poll(cx) {
            Poll::Ready(value) => {
                // Mark the task as completed
                this.task_manager.complete_task(this.task_id);
                Poll::Ready(Ok(value))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Async bridge for converting Futures to JavaScript callbacks
pub struct AsyncBridge {
    /// Reference to the AsyncTaskManager
    task_manager: Arc<AsyncTaskManager>,
}

impl AsyncBridge {
    /// Create a new AsyncBridge
    pub fn new(task_manager: Arc<AsyncTaskManager>) -> Self {
        Self { task_manager }
    }

    /// Spawn a cancellable async task
    ///
    /// This method wraps a Future in a CancellableFuture and registers it with the
    /// AsyncTaskManager. The future will be cancelled if the context is dropped.
    pub fn spawn_cancellable<T, F>(
        &self,
        future: F,
        callback: AsyncCallback<T>,
    ) -> u64
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let task_id = self.task_manager.register_task(TaskPriority::Cancellable);
        let cancellable_future = CancellableFuture::new(future, task_id, self.task_manager.clone());

        // Spawn the future on a new thread (in a real implementation, this would use a runtime)
        let task_manager = self.task_manager.clone();
        std::thread::spawn(move || {
            // Mark the task as running
            task_manager.start_task(task_id);

            // Create a simple runtime to poll the future
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);
            let mut future = cancellable_future;

            // Poll the future to completion
            let result = loop {
                match Pin::new(&mut future).poll(&mut cx) {
                    Poll::Ready(result) => break result,
                    Poll::Pending => {
                        // In a real implementation, we would yield here
                        std::thread::yield_now();
                    }
                }
            };

            // Only call the callback if the task was not cancelled or timed out
            match &result {
                Ok(_) => callback(result),
                Err(AsyncBridgeError::Cancelled) | Err(AsyncBridgeError::TimedOut) => {
                    // Task was cancelled or timed out, don't call callback
                }
                Err(_) => callback(result),
            }
        });

        task_id
    }

    /// Spawn a non-cancellable async task
    ///
    /// This method wraps a Future in a CancellableFuture with NonCancellable priority.
    /// The future will NOT be cancelled if the context is dropped.
    pub fn spawn_non_cancellable<T, F>(
        &self,
        future: F,
        callback: AsyncCallback<T>,
    ) -> u64
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let task_id = self.task_manager.register_task(TaskPriority::NonCancellable);
        let cancellable_future = CancellableFuture::new(future, task_id, self.task_manager.clone());

        // Spawn the future on a new thread
        let task_manager = self.task_manager.clone();
        std::thread::spawn(move || {
            // Mark the task as running
            task_manager.start_task(task_id);

            // Create a simple runtime to poll the future
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);
            let mut future = cancellable_future;

            // Poll the future to completion
            let result = loop {
                match Pin::new(&mut future).poll(&mut cx) {
                    Poll::Ready(result) => break result,
                    Poll::Pending => {
                        // In a real implementation, we would yield here
                        std::thread::yield_now();
                    }
                }
            };

            // Only call the callback if the task was not cancelled or timed out
            match &result {
                Ok(_) => callback(result),
                Err(AsyncBridgeError::Cancelled) | Err(AsyncBridgeError::TimedOut) => {
                    // Task was cancelled or timed out, don't call callback
                }
                Err(_) => callback(result),
            }
        });

        task_id
    }

    /// Spawn a timeout async task
    ///
    /// This method wraps a Future in a CancellableFuture with Timeout priority.
    /// The future will be cancelled if it doesn't complete within the specified timeout.
    pub fn spawn_with_timeout<T, F>(
        &self,
        future: F,
        callback: AsyncCallback<T>,
        timeout_ms: u64,
    ) -> u64
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let task_id = self
            .task_manager
            .register_task(TaskPriority::Timeout(timeout_ms));
        let cancellable_future = CancellableFuture::new(future, task_id, self.task_manager.clone());

        // Spawn the future on a new thread
        let task_manager = self.task_manager.clone();
        std::thread::spawn(move || {
            // Mark the task as running
            task_manager.start_task(task_id);

            // Create a simple runtime to poll the future
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);
            let mut future = cancellable_future;

            // Poll the future to completion
            let result = loop {
                match Pin::new(&mut future).poll(&mut cx) {
                    Poll::Ready(result) => break result,
                    Poll::Pending => {
                        // In a real implementation, we would yield here
                        std::thread::yield_now();
                    }
                }
            };

            // Only call the callback if the task was not cancelled or timed out
            match &result {
                Ok(_) => callback(result),
                Err(AsyncBridgeError::Cancelled) | Err(AsyncBridgeError::TimedOut) => {
                    // Task was cancelled or timed out, don't call callback
                }
                Err(_) => callback(result),
            }
        });

        task_id
    }

    /// Cancel a specific task
    pub fn cancel_task(&self, task_id: u64) -> bool {
        self.task_manager.cancel_task(task_id)
    }

    /// Get the task manager
    pub fn task_manager(&self) -> &AsyncTaskManager {
        &self.task_manager
    }
}

/// Macro for creating async bridges with automatic task management
///
/// # Usage
///
/// ```rust
/// use mquickjs_rs::async_bridge::{AsyncBridge, AsyncCallback};
/// use std::sync::Arc;
///
/// let task_manager = Arc::new(mquickjs_rs::async_task::AsyncTaskManager::new());
/// let bridge = AsyncBridge::new(task_manager);
///
/// // Spawn a cancellable task
/// let task_id = bridge.spawn_cancellable(
///     async { 42 },
///     Box::new(|result| {
///         println!("Result: {:?}", result);
///     }),
/// );
/// ```
#[macro_export]
macro_rules! js_async {
    // Cancellable task
    (cancellable $bridge:expr, $future:expr, $callback:expr) => {
        $bridge.spawn_cancellable($future, $callback)
    };

    // Non-cancellable task
    (non_cancellable $bridge:expr, $future:expr, $callback:expr) => {
        $bridge.spawn_non_cancellable($future, $callback)
    };

    // Timeout task
    (timeout $bridge:expr, $future:expr, $callback:expr, $timeout_ms:expr) => {
        $bridge.spawn_with_timeout($future, $callback, $timeout_ms)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_async_bridge_creation() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        assert_eq!(bridge.task_manager().active_task_count(), 0);
    }

    #[test]
    fn test_spawn_cancellable_task() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();
        
        let task_id = bridge.spawn_cancellable(
            async { 42 },
            Box::new(move |result| {
                assert_eq!(result.unwrap(), 42);
                callback_called_clone.store(true, Ordering::SeqCst);
            }),
        );
        
        // Wait for the task to complete
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(callback_called.load(Ordering::SeqCst));
        assert_eq!(
            bridge.task_manager().get_task_status(task_id),
            Some(TaskStatus::Completed)
        );
    }

    #[test]
    fn test_spawn_non_cancellable_task() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();
        
        let task_id = bridge.spawn_non_cancellable(
            async { 42 },
            Box::new(move |result| {
                assert_eq!(result.unwrap(), 42);
                callback_called_clone.store(true, Ordering::SeqCst);
            }),
        );
        
        // Wait for the task to complete
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(callback_called.load(Ordering::SeqCst));
        assert_eq!(
            bridge.task_manager().get_task_status(task_id),
            Some(TaskStatus::Completed)
        );
    }

    #[test]
    fn test_spawn_timeout_task() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();
        
        let task_id = bridge.spawn_with_timeout(
            async { 42 },
            Box::new(move |result| {
                assert_eq!(result.unwrap(), 42);
                callback_called_clone.store(true, Ordering::SeqCst);
            }),
            5000, // 5 second timeout
        );
        
        // Wait for the task to complete
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(callback_called.load(Ordering::SeqCst));
        assert_eq!(
            bridge.task_manager().get_task_status(task_id),
            Some(TaskStatus::Completed)
        );
    }

    #[test]
    fn test_cancel_task() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let task_id = bridge.spawn_cancellable(
            async {
                // Simulate a long-running task
                std::thread::sleep(Duration::from_secs(10));
                42
            },
            Box::new(|_result| {
                // This should not be called
                panic!("Callback should not be called for cancelled task");
            }),
        );
        
        // Cancel the task
        assert!(bridge.cancel_task(task_id));
        
        // Wait a bit to ensure the task is cancelled
        std::thread::sleep(Duration::from_millis(100));
        
        assert_eq!(
            bridge.task_manager().get_task_status(task_id),
            Some(TaskStatus::Cancelled)
        );
    }

    #[test]
    fn test_cancellable_future() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let task_id = task_manager.register_task(TaskPriority::Cancellable);
        
        let future = CancellableFuture::new(
            async { 42 },
            task_id,
            task_manager.clone(),
        );
        
        // The future should not be cancelled initially
        assert!(!future.is_cancelled());
        
        // Cancel the task
        task_manager.cancel_task(task_id);
        
        // The future should now be cancelled
        assert!(future.is_cancelled());
    }

    #[test]
    fn test_async_bridge_error_display() {
        let errors = vec![
            AsyncBridgeError::Cancelled,
            AsyncBridgeError::TimedOut,
            AsyncBridgeError::ContextDropped,
            AsyncBridgeError::FutureError("test error".to_string()),
        ];
        
        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_js_async_macro() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();
        
        let task_id = js_async!(
            cancellable bridge,
            async { 42 },
            Box::new(move |result| {
                assert_eq!(result.unwrap(), 42);
                callback_called_clone.store(true, Ordering::SeqCst);
            })
        );
        
        // Wait for the task to complete
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(callback_called.load(Ordering::SeqCst));
        assert_eq!(
            bridge.task_manager().get_task_status(task_id),
            Some(TaskStatus::Completed)
        );
    }

    #[test]
    fn test_multiple_tasks() {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        
        let mut task_ids = Vec::new();
        
        // Spawn multiple tasks
        for i in 0..10 {
            let task_id = bridge.spawn_cancellable(
                async move { i },
                Box::new(move |_result| {}),
            );
            task_ids.push(task_id);
        }
        
        // Wait for all tasks to complete
        std::thread::sleep(Duration::from_millis(200));
        
        // All tasks should be completed
        for task_id in task_ids {
            assert_eq!(
                bridge.task_manager().get_task_status(task_id),
                Some(TaskStatus::Completed)
            );
        }
    }
}
