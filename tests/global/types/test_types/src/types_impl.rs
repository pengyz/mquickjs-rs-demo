use crate::api::TestTypesSingleton;

pub struct DefaultTestTypesSingleton;

impl TestTypesSingleton for DefaultTestTypesSingleton {
    fn echo_bool(&mut self, v: bool) -> bool {
        v
    }

    fn echo_i32(&mut self, v: i32) -> i32 {
        v
    }

    fn echo_f64(&mut self, v: f64) -> f64 {
        v
    }

    fn echo_f32(&mut self, v: f32) -> f32 {
        v
    }

    fn echo_i64(&mut self, v: i64) -> i64 {
        v
    }

    fn echo_string(&mut self, v: String) -> String {
        v
    }

    fn echo_string_nullable(&mut self, v: Option<String>) -> Option<String> {
        v
    }

    fn echo_i32_nullable(&mut self, v: Option<i32>) -> Option<i32> {
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
        let raw = env.str("ok").expect("env.str should succeed").as_raw();
        Some(env.return_safe(env.scope().value(raw)))
    }

    fn maybe_union_any<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        v: Option<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>>,
    ) -> Option<crate::api::global::union::UnionI32String> {
        let Some(v) = v else {
            return None;
        };

        // Decode only string/i32 to validate Optional(any) param decoding.
        // Other types map to None.
        let raw = v.as_raw();
        let v_str = env.scope().value(raw);
        if let Ok(s) = env.get_string(v_str) {
            return Some(crate::api::global::union::UnionI32String::String(s));
        }

        let v_num = env.scope().value(raw);
        if let Ok(n) = env.get_number(v_num) {
            if n.fract() == 0.0 {
                return Some(crate::api::global::union::UnionI32String::I32(n as i32));
            }
        }

        // NOTE: treat numeric but non-integer as string for this test.
        // This keeps the test focused on optional(any) param decode / optional(union) return,
        // without relying on exact numeric tagging in this engine.
        let v_num2 = env.scope().value(raw);
        if let Ok(n) = env.get_number(v_num2) {
            return Some(crate::api::global::union::UnionI32String::String(
                n.to_string(),
            ));
        }

        None
    }

    fn echo_string_or_i32(
        &mut self,
        v: crate::api::global::union::UnionI32String,
    ) -> crate::api::global::union::UnionI32String {
        v
    }

    fn echo_string_or_i32_nullable(
        &mut self,
        v: Option<crate::api::global::union::UnionI32String>,
    ) -> Option<crate::api::global::union::UnionI32String> {
        v
    }

    fn echo_f64_new(&mut self, v: f64) -> f64 {
        v
    }

    fn echo_f64_new_nullable(&mut self, v: Option<f64>) -> Option<f64> {
        v
    }

    fn sum_map_i32_string(&mut self, v: std::collections::HashMap<i32, String>) -> i32 {
        v.iter().map(|(k, s)| k + (s.len() as i32)).sum()
    }

    fn sum_map_bool_i32(&mut self, v: std::collections::HashMap<bool, i32>) -> i32 {
        v.iter().map(|(k, n)| (if *k { 1 } else { 0 }) + *n).sum()
    }

    fn sum_map_i64_i32(&mut self, v: std::collections::HashMap<i64, i32>) -> i32 {
        v.iter().map(|(k, n)| ((*k % 1000) as i32) + *n).sum()
    }

    fn make_map_i32_string(&mut self) -> std::collections::HashMap<i32, String> {
        let mut out = std::collections::HashMap::new();
        out.insert(1, "a".to_string());
        out.insert(-2, "bb".to_string());
        out
    }
}

pub fn create_test_types_singleton() -> Box<dyn TestTypesSingleton> {
    Box::new(DefaultTestTypesSingleton)
}
