mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestJsFieldsSingleton;

    pub use crate::js_fields_impl::DefaultTestJsFieldsSingleton;
}

mod js_fields_impl;

pub use js_fields_impl::ridl_create_test_js_fields_singleton;
