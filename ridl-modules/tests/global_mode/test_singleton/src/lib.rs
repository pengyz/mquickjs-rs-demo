mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestPingSingleton;

    pub use crate::singleton_impl::DefaultTestPingSingleton;
    pub use crate::singleton_impl::create_test_ping_singleton;
}

mod singleton_impl;
