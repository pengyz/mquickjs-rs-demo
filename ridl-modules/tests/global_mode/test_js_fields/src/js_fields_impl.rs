use crate::api::TestjsfieldsSingleton;

pub struct DefaultTestJsFieldsSingleton;

impl TestjsfieldsSingleton for DefaultTestJsFieldsSingleton {
    fn getnullany(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testjsfields_singleton() -> Box<dyn TestjsfieldsSingleton> {
    Box::new(DefaultTestJsFieldsSingleton)
}
