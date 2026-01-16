mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestLiteralsSingleton;

    pub use crate::literals_impl::DefaultTestLiteralsSingleton;
    pub use crate::literals_impl::create_test_literals_singleton;
}

mod literals_impl;
