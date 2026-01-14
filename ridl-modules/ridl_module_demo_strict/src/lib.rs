// Keep a private module to contain all includes.
mod generated;

pub mod api {}

pub mod glue {
    pub use crate::generated::glue::*;
}

pub mod impls;

// Re-export glue symbols for C side registration / lookup if needed.
pub use glue::*;

pub use glue::{initialize_module, ridl_module_context_init};
