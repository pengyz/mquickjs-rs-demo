mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestLiteralsSingleton;

    pub use crate::literals_impl::DefaultTestLiteralsSingleton;
}

mod literals_impl;

pub use literals_impl::ridl_create_test_literals_singleton;
