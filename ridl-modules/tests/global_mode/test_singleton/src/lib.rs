mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestsingletonSingleton;

    pub use crate::singleton_impl::DefaultTestSingletonSingleton;
}

mod singleton_impl;

pub use singleton_impl::ridl_create_testsingleton_singleton;
