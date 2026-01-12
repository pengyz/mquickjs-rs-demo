mod generated;

pub mod impls;

// Module-local singleton trait exports expected by generated glue.
pub use crate::impls::*;

mod __ridl_module_api {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_api.rs"));
}

pub use __ridl_module_api::{initialize_module, ridl_module_context_init};

// Re-export glue symbols for C side registration / lookup if needed.
pub use generated::glue::*;
