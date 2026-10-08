mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestStructEnumSingleton;

    pub use crate::struct_enum_impl::DefaultTestStructEnumSingleton;
    pub use crate::struct_enum_impl::create_test_struct_enum_singleton;
}

mod struct_enum_impl;
