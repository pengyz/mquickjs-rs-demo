mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestFnSingleton;

    pub use crate::fn_impl::DefaultTestFnSingleton;
}

mod fn_impl;

pub use fn_impl::ridl_create_test_fn_singleton;
