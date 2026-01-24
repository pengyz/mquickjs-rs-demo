mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ConsoleSingleton;

    pub use crate::stdlib_impl::DefaultConsoleSingleton;

    pub use crate::stdlib_impl::create_console_singleton;
}

mod stdlib_impl;
