mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestVarargsSingleton;

    pub use crate::varargs_impl::DefaultTestVarargsSingleton;
    pub use crate::varargs_impl::create_test_varargs_singleton;
}

mod varargs_impl;
