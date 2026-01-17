mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestclassSingleton;

    pub use crate::class_impl::DefaultTestClassSingleton;
}

mod class_impl;

pub use class_impl::ridl_create_testclass_singleton;
