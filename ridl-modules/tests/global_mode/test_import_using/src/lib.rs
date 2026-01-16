mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestImportUsingSingleton;

    pub use crate::import_using_impl::DefaultTestImportUsingSingleton;
    pub use crate::import_using_impl::create_test_import_using_singleton;
}

mod import_using_impl;
