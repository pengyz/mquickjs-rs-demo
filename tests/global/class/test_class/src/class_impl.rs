use crate::api::{TestclassSingleton, UserClass};

pub struct DefaultTestClassSingleton;

impl TestclassSingleton for DefaultTestClassSingleton {
    fn makeuser(
        &mut self,
        _ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext,
        _args: Vec<mquickjs_rs::mquickjs_ffi::JSValue>,
    ) {
    }
}

struct DefaultUser {
    name: String,
}

impl UserClass for DefaultUser {
    fn getname(&mut self) -> String {
        self.name.clone()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_testclass_singleton() -> Box<dyn TestclassSingleton> {
    Box::new(DefaultTestClassSingleton)
}
