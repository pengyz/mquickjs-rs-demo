pub mod impls {
    pub use crate::stdlib_impl::*;
}

#[path = "../stdlib_impl.rs"]
mod stdlib_impl;

mod __ridl_module_api {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_api.rs"));
}

pub use __ridl_module_api::initialize_module;

// Re-export glue symbols for C side registration / lookup if needed.
mod generated {
    pub mod glue {
        include!(concat!(env!("OUT_DIR"), "/stdlib_glue.rs"));
    }
    pub mod symbols {
        include!(concat!(env!("OUT_DIR"), "/stdlib_symbols.rs"));
    }
}

pub use generated::glue::*;