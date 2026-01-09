pub(crate) mod glue {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo_glue.rs"));
}

pub(crate) mod symbols {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo_symbols.rs"));
}

pub(crate) mod register {
    // ridl-tool's Rust glue currently doesn't generate a per-module register() function.
    // Registration is compile-time via C stdlib tables.
    pub(crate) unsafe fn register(_ctx: *mut mquickjs_sys::JSContext) {}
}
