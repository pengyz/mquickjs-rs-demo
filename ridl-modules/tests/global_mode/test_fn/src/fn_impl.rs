use crate::api::TestfnSingleton;

pub struct DefaultTestFnSingleton;

impl TestfnSingleton for DefaultTestFnSingleton {
    fn addint(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // Implemented by glue for now; keep as no-op.
    }

}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testfn_singleton() -> Box<dyn TestfnSingleton> {
    Box::new(DefaultTestFnSingleton)
}
