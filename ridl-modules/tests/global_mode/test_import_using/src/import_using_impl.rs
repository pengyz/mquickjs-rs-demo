use crate::api::TestimportusingSingleton;

pub struct DefaultTestImportUsingSingleton;

impl TestimportusingSingleton for DefaultTestImportUsingSingleton {
    fn ping(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testimportusing_singleton() -> Box<dyn TestimportusingSingleton> {
    Box::new(DefaultTestImportUsingSingleton)
}
