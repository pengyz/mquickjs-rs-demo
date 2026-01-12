pub mod context;
pub mod ridl_context_init;
pub mod test_runner;

pub mod ctx_ext {
    include!(concat!(env!("OUT_DIR"), "/ridl_ctx_ext.rs"));
}

pub mod ridl_initialize {
    include!(concat!(env!("OUT_DIR"), "/ridl_initialize.rs"));
}

pub use context::Context;
