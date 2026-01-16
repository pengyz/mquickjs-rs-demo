use crate::api::TesttypesSingleton;

pub struct DefaultTestTypesSingleton;

impl TesttypesSingleton for DefaultTestTypesSingleton {
    fn echoany(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // v1 glue handles conversions and pushes return values via JS directly.
        // This module uses only no-return smoke APIs for now.
    }

}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testtypes_singleton() -> Box<dyn TesttypesSingleton> {
    Box::new(DefaultTestTypesSingleton)
}
