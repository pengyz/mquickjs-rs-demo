pub mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}

pub mod glue {
    include!(concat!(env!("OUT_DIR"), "/glue.rs"));
}

pub mod impls {
    pub use crate::api::ConsoleSingleton;

    pub use crate::stdlib_impl::DefaultConsoleSingleton;
    pub use crate::stdlib_impl::create_console_singleton;
}

// Erased singleton vtables consumed by the app-side aggregated ridl_context_init.
pub static RIDL_CONSOLE_SINGLETON_VT: mquickjs_rs::ridl_runtime::RidlErasedSingletonVTable =
    mquickjs_rs::ridl_runtime::RidlErasedSingletonVTable {
        create: ridl_console_singleton_create,
        drop: ridl_console_singleton_drop,
    };

unsafe extern "C" fn ridl_console_singleton_create() -> *mut core::ffi::c_void {
    let b: Box<dyn impls::ConsoleSingleton> = impls::create_console_singleton();
    // Store a pointer to the Box (thin pointer), so it can round-trip through c_void safely.
    Box::into_raw(Box::new(b)) as *mut core::ffi::c_void
}

unsafe extern "C" fn ridl_console_singleton_drop(p: *mut core::ffi::c_void) {
    if !p.is_null() {
        unsafe {
            let holder: Box<Box<dyn impls::ConsoleSingleton>> = Box::from_raw(p as *mut _);
            drop(holder);
        }
    }
}

#[path = "../stdlib_impl.rs"]
mod stdlib_impl;

// Re-export glue symbols for C side registration / lookup if needed.
pub use glue::*;

pub use glue::{initialize_module, ridl_module_context_init};
