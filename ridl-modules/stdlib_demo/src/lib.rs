mod generated;

pub mod impls;

mod __ridl_module_api {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_api.rs"));
}

pub use __ridl_module_api::initialize_module;

pub fn register(_ctx: *mut mquickjs_sys::JSContext) {
    // Registration is compile-time via C stdlib tables.
}

// Re-export glue symbols for C side registration / lookup if needed.
pub use generated::glue::*;
