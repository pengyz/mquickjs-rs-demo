mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestTypesSingleton;

    pub use crate::types_impl::DefaultTestTypesSingleton;
}

mod types_impl;

pub use types_impl::ridl_create_test_types_singleton;
