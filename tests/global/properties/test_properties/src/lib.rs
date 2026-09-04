mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::PropertyTestSingletonSingleton;
    pub use crate::api::PropertyTestClassClass;
    
    pub use crate::singleton_impl::DefaultPropertyTestSingleton;
    pub use crate::singleton_impl::create_property_test_singleton_singleton;
    
    pub use crate::class_impl::DefaultPropertyTestClass;
    pub use crate::class_impl::create_property_test_class_class;
    pub use crate::class_impl::property_test_class_constructor;
}

mod singleton_impl;
mod class_impl;