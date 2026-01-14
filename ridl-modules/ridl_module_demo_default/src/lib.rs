// Keep a private module to contain all includes.
mod generated;

pub mod api {
    pub use crate::generated::api::*;
}

pub mod glue {
    pub use crate::generated::glue::*;
}

pub mod impls;

// Erased singleton vtables consumed by the app-side aggregated ridl_context_init.
pub static RIDL_DEMO_SINGLETON_VT: mquickjs_rs::ridl_runtime::RidlErasedSingletonVTable =
    mquickjs_rs::ridl_runtime::RidlErasedSingletonVTable {
        create: ridl_demo_singleton_create,
        drop: ridl_demo_singleton_drop,
    };

unsafe extern "C" fn ridl_demo_singleton_create() -> *mut core::ffi::c_void {
    let b: Box<dyn api::DemoSingleton> = impls::create_demo_singleton();
    // Store a pointer to the Box (thin pointer), so it can round-trip through c_void safely.
    Box::into_raw(Box::new(b)) as *mut core::ffi::c_void
}

unsafe extern "C" fn ridl_demo_singleton_drop(p: *mut core::ffi::c_void) {
    if !p.is_null() {
        unsafe {
            let holder: Box<Box<dyn api::DemoSingleton>> = Box::from_raw(p as *mut _);
            drop(holder);
        }
    }
}

// Re-export glue symbols for C side registration / lookup if needed.
pub use glue::*;

pub use glue::{initialize_module, ridl_module_context_init};
