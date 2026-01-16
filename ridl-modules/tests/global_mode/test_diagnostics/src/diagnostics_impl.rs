use crate::api::TestDiagnosticsSingleton;

pub struct DefaultTestDiagnosticsSingleton;

impl TestDiagnosticsSingleton for DefaultTestDiagnosticsSingleton {
    fn ok(&mut self) -> bool {
        true
    }
}

pub fn create_test_diagnostics_singleton() -> Box<dyn TestDiagnosticsSingleton> {
    Box::new(DefaultTestDiagnosticsSingleton)
}
