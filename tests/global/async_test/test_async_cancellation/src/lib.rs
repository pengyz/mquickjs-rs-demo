mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::AsyncTestSingletonSingleton;

    pub use crate::singleton_impl::DefaultAsyncTestSingletonSingleton;
    pub use crate::singleton_impl::create_async_test_singleton_singleton;
}

mod singleton_impl;
