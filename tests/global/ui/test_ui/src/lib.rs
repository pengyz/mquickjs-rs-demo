mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::UISingleton;
    pub use crate::singleton_impl::DefaultUI;
    pub use crate::singleton_impl::create_ui_singleton;
}

mod singleton_impl;