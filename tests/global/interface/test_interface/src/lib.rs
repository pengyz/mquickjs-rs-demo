mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ShapeServiceSingleton;
    pub use crate::singleton_impl::DefaultShapeService;
    pub use crate::singleton_impl::create_shape_service_singleton;
}

mod singleton_impl;