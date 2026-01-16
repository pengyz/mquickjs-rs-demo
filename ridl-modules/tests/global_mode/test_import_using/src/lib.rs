mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestImportUsingSingleton;

    pub use crate::import_using_impl::DefaultTestImportUsingSingleton;
}

mod import_using_impl;

pub use import_using_impl::ridl_create_test_import_using_singleton;
