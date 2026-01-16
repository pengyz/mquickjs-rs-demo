mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestFnSingleton;

    pub use crate::fn_impl::DefaultTestFnSingleton;
    pub use crate::fn_impl::create_test_fn_singleton;
}

mod fn_impl;
