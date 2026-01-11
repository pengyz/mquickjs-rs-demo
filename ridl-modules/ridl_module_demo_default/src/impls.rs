#[path = "../ridl_module_demo_default_impl.rs"]
mod ridl_module_demo_default_impl;

pub use ridl_module_demo_default_impl::*;

// Re-export generated singleton traits so glue can use stable paths: crate::impls::<Name>Singleton.
pub use crate::generated::impls::DemoSingleton;
pub use crate::generated::impls::create_demo_singleton;
