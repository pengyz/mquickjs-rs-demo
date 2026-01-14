mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::ConsoleSingleton;

    pub use crate::stdlib_impl::DefaultConsoleSingleton;
}

mod stdlib_impl;

pub use stdlib_impl::ridl_create_console_singleton;

