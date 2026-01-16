mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestTypesSingleton;

    pub use crate::types_impl::DefaultTestTypesSingleton;
    pub use crate::types_impl::create_test_types_singleton;
}

mod types_impl;
