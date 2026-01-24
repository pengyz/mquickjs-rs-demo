mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestClassSingleton;

    pub fn create_test_class_singleton() -> Box<dyn TestClassSingleton> {
        Box::new(crate::class_impl::DefaultTestClassSingleton)
    }

    pub fn user_constructor() -> Box<dyn crate::api::UserClass> {
        Box::new(crate::class_impl::DefaultUser {
            name: String::new(),
        })
    }
}

mod class_impl;
