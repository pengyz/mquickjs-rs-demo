use crate::api::TestTypesSingleton;

pub struct DefaultTestTypesSingleton;

impl TestTypesSingleton for DefaultTestTypesSingleton {
    fn echo_bool(&mut self, v: bool) -> bool {
        v
    }

    fn echo_int(&mut self, v: i32) -> i32 {
        v
    }

    fn echo_double(&mut self, v: f64) -> f64 {
        v
    }

    fn echo_string(&mut self, v: String) -> String {
        v
    }

    fn echo_string_nullable(&mut self, v: Option<String>) -> Option<String> {
        v
    }

    fn echo_int_nullable(&mut self, v: Option<i32>) -> Option<i32> {
        v
    }

    fn echo_any(
        &mut self,
        v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> mquickjs_rs::handles::global::Global<mquickjs_rs::handles::local::Value> {
        // V1: any is passed through as a handle; do not normalize.
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            panic!("echoAny must run inside JS context");
        };
        let scope = h.enter_scope();
        mquickjs_rs::handles::global::Global::new(&scope, v)
    }

    fn echo_string_or_int(
        &mut self,
        v: crate::api::global::union::UnionIntString,
    ) -> crate::api::global::union::UnionIntString {
        v
    }

    fn echo_string_or_int_nullable(
        &mut self,
        v: Option<crate::api::global::union::UnionIntString>,
    ) -> Option<crate::api::global::union::UnionIntString> {
        v
    }
}

pub fn create_test_types_singleton() -> Box<dyn TestTypesSingleton> {
    Box::new(DefaultTestTypesSingleton)
}
