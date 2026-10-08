mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestSingletonVarSingleton;

    pub use crate::singleton_var_impl::DefaultTestSingletonVarSingleton;
    pub use crate::singleton_var_impl::create_test_singleton_var_singleton;
}

mod singleton_var_impl;
