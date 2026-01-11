pub mod context;
pub mod ridl_context_init;

pub mod ctx_ext {
    include!(concat!(env!("OUT_DIR"), "/ridl_ctx_ext.rs"));
}

pub use context::Context;
