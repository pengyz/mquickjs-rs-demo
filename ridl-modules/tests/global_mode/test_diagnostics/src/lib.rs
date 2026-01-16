mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestdiagnosticsSingleton;

    pub use crate::diagnostics_impl::DefaultTestDiagnosticsSingleton;
}

mod diagnostics_impl;

pub use diagnostics_impl::ridl_create_testdiagnostics_singleton;
