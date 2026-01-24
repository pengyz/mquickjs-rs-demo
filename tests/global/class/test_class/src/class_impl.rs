use crate::api::{TestClassSingleton, UserClass};

pub struct DefaultTestClassSingleton;

impl TestClassSingleton for DefaultTestClassSingleton {
    fn make_user(&mut self, name: String) -> Box<dyn crate::api::UserClass> {
        Box::new(DefaultUser { name })
    }
}

pub struct DefaultUser {
    pub name: String,
}

impl UserClass for DefaultUser {
    fn get_name(&mut self) -> String {
        self.name.clone()
    }

    fn echo_any<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        v: Option<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>>,
    ) -> Option<mquickjs_rs::handles::return_safe::ReturnAny> {
        match v {
            None => None,
            Some(v) => Some(env.return_safe(env.scope().value(v.as_raw()))),
        }
    }
}
