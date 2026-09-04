use crate::api::AsyncTestSingletonSingleton;
use mquickjs_rs::async_bridge::{AsyncBridge, AsyncCallback};
use mquickjs_rs::async_task::AsyncTaskManager;
use std::sync::Arc;

pub struct DefaultAsyncTestSingletonSingleton {
    bridge: AsyncBridge,
}

impl DefaultAsyncTestSingletonSingleton {
    pub fn new() -> Self {
        let task_manager = Arc::new(AsyncTaskManager::new());
        let bridge = AsyncBridge::new(task_manager);
        Self { bridge }
    }
}

impl AsyncTestSingletonSingleton for DefaultAsyncTestSingletonSingleton {
    fn non_cancellable_task(&mut self, cb: AsyncCallback<String>) {
        self.bridge.spawn_non_cancellable(
            async {
                // Simulate some async work
                std::thread::sleep(std::time::Duration::from_millis(100));
                "non-cancellable result".to_string()
            },
            cb,
        );
    }

    fn timeout_task(&mut self, cb: AsyncCallback<String>) {
        self.bridge.spawn_with_timeout(
            async {
                // Simulate some async work
                std::thread::sleep(std::time::Duration::from_millis(100));
                "timeout result".to_string()
            },
            cb,
            1000, // 1 second timeout
        );
    }

    fn cancellable_task(&mut self, cb: AsyncCallback<String>) {
        self.bridge.spawn_cancellable(
            async {
                // Simulate some async work
                std::thread::sleep(std::time::Duration::from_millis(100));
                "cancellable result".to_string()
            },
            cb,
        );
    }
}

pub fn create_async_test_singleton_singleton() -> Box<dyn AsyncTestSingletonSingleton> {
    Box::new(DefaultAsyncTestSingletonSingleton::new())
}
