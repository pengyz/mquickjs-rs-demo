use crate::api::TestTypesSingleton;

pub struct DefaultTestTypesSingleton;

impl TestTypesSingleton for DefaultTestTypesSingleton {
    fn echo_any(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // v1 glue handles conversions and pushes return values via JS directly.
        // This module uses only no-return smoke APIs for now.
    }

}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_types_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestTypesSingleton> = Box::new(DefaultTestTypesSingleton);
    let holder: Box<Box<dyn TestTypesSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
