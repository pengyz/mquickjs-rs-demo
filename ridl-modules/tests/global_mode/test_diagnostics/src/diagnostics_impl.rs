use crate::api::TestdiagnosticsSingleton;

pub struct DefaultTestDiagnosticsSingleton;

impl TestdiagnosticsSingleton for DefaultTestDiagnosticsSingleton {
    fn ok(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testdiagnostics_singleton() -> Box<dyn TestdiagnosticsSingleton> {
    Box::new(DefaultTestDiagnosticsSingleton)
}
