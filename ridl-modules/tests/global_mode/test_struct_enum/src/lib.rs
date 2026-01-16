pub mod impls {
    pub use crate::api::TestStructEnumSingleton;

    pub use crate::struct_enum_impl::DefaultTestStructEnumSingleton;
}

mod struct_enum_impl;

pub use struct_enum_impl::ridl_create_test_struct_enum_singleton;
