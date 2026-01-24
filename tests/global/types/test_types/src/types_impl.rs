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

    fn echo_any<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        v: mquickjs_rs::handles::local::Local<'ctx, mquickjs_rs::handles::local::Value>,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        env.return_safe(v)
    }

    fn maybe_any(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        v: bool,
    ) -> Option<mquickjs_rs::handles::return_safe::ReturnAny> {
        if !v {
            return None;
        }
        let raw = env
            .str("ok")
            .expect("env.str should succeed")
            .as_raw();
        Some(env.return_safe(env.scope().value(raw)))
    }

    fn maybe_union_any<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        v: Option<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>>,
    ) -> Option<crate::api::global::union::UnionIntString> {
        let Some(v) = v else {
            return None;
        };

        // Decode only string/int to validate Optional(any) param decoding.
        // Other types map to None.
        let raw = v.as_raw();
        let v_str = env.scope().value(raw);
        if let Ok(s) = env.get_string(v_str) {
            return Some(crate::api::global::union::UnionIntString::String(s));
        }

        let v_num = env.scope().value(raw);
        if let Ok(n) = env.get_number(v_num) {
            if n.fract() == 0.0 {
                return Some(crate::api::global::union::UnionIntString::Int(n as i32));
            }
        }

        // NOTE: treat numeric but non-integer as string for this test.
        // This keeps the test focused on optional(any) param decode / optional(union) return,
        // without relying on exact numeric tagging in this engine.
        let v_num2 = env.scope().value(raw);
        if let Ok(n) = env.get_number(v_num2) {
            return Some(crate::api::global::union::UnionIntString::String(n.to_string()));
        }

        None
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
