mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ButtonClass;

    pub fn button_constructor() -> Box<dyn ButtonClass> {
        Box::new(crate::bridge_impl::DefaultButton::new())
    }
}

mod bridge_impl;
