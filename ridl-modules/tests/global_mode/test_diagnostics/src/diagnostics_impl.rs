use crate::api::TestDiagnosticsSingleton;

pub struct DefaultTestDiagnosticsSingleton;

impl TestDiagnosticsSingleton for DefaultTestDiagnosticsSingleton {
    fn ok(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_diagnostics_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestDiagnosticsSingleton> = Box::new(DefaultTestDiagnosticsSingleton);
    let holder: Box<Box<dyn TestDiagnosticsSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
