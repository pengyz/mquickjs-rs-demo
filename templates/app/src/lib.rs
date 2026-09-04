mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::MyServiceSingleton;
    pub use crate::my_service_impl::DefaultMyService;
    pub use crate::my_service_impl::create_my_service_singleton;
}

mod my_service_impl;