mod generated;

pub mod impls;

mod __ridl_module_api {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_api.rs"));
}

pub use __ridl_module_api::initialize_module;

// Re-export glue symbols for C side registration / lookup if needed.
pub use generated::glue::*;
