mquickjs_rs::ridl_include_module!();

pub mod impls {
    pub use crate::api::TestDiagnosticsSingleton;

    pub use crate::diagnostics_impl::DefaultTestDiagnosticsSingleton;
    pub use crate::diagnostics_impl::create_test_diagnostics_singleton;
}

mod diagnostics_impl;
