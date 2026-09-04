mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ConstantTestSingletonSingleton;
    pub use crate::api::ConstantTestClassClass;
    
    pub use crate::singleton_impl::DefaultConstantTestSingleton;
    pub use crate::singleton_impl::create_constant_test_singleton_singleton;
    
    pub use crate::class_impl::DefaultConstantTestClass;
    pub use crate::class_impl::create_constant_test_class_class;
    pub use crate::class_impl::constant_test_class_constructor;
}

mod singleton_impl;
mod class_impl;