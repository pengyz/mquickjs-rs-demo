use crate::api::TestImportUsingSingleton;

pub struct DefaultTestImportUsingSingleton;

impl TestImportUsingSingleton for DefaultTestImportUsingSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_test_import_using_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn TestImportUsingSingleton> = Box::new(DefaultTestImportUsingSingleton);
    let holder: Box<Box<dyn TestImportUsingSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}
