mod generated;

pub mod impls;

pub fn ensure_linked() {
    generated::symbols::ensure_symbols();
}

pub fn register(_ctx: *mut mquickjs_sys::JSContext) {
    // Registration is compile-time via C stdlib tables.
}

// Re-export glue symbols for C side registration / lookup if needed.
pub use generated::glue::*;
