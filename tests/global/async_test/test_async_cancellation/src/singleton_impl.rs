use crate::api::AsyncTestSingletonSingleton;

pub struct DefaultAsyncTestSingletonSingleton;

// Wrapper to make raw pointer Send
struct SendPtr(*mut Box<dyn AsyncTestSingletonSingleton>);
unsafe impl Send for SendPtr {}

impl AsyncTestSingletonSingleton for DefaultAsyncTestSingletonSingleton {
    fn non_cancellable_task(&mut self) -> String {
        // Simulate some async work
        std::thread::sleep(std::time::Duration::from_millis(100));
        "non-cancellable result".to_string()
    }

    fn timeout_task(&mut self) -> String {
        // Simulate some async work
        std::thread::sleep(std::time::Duration::from_millis(100));
        "timeout result".to_string()
    }

    fn cancellable_task(&mut self) -> String {
        // Simulate some async work
        std::thread::sleep(std::time::Duration::from_millis(100));
        "cancellable result".to_string()
    }
}

pub fn create_async_test_singleton_singleton() -> Box<dyn AsyncTestSingletonSingleton> {
    Box::new(DefaultAsyncTestSingletonSingleton)
}
