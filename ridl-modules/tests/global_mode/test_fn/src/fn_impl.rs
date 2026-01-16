use crate::api::TestFnSingleton;

pub struct DefaultTestFnSingleton;

impl TestFnSingleton for DefaultTestFnSingleton {
    fn add_int(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
        // Implemented by glue for now; keep as no-op.
    }

}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_fn_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestFnSingleton> = Box::new(DefaultTestFnSingleton);
    let holder: Box<Box<dyn TestFnSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
