use crate::api::TestFnSingleton;

pub struct DefaultTestFnSingleton;

impl TestFnSingleton for DefaultTestFnSingleton {
    fn add_i32(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn echo_any<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        v: mquickjs_rs::handles::local::Local<'ctx, mquickjs_rs::handles::local::Value>,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        env.return_safe(v)
    }

    fn make_any_string(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        s: String,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        let raw = env.str(&s).expect("env.str should succeed").as_raw();
        env.return_safe(env.scope().value(raw))
    }

    fn any_to_string(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> String {
        env.get_string(v)
            .expect("any_to_string expects a string value")
    }

    fn make_array_with_len(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        len: i32,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        let len: u32 = len.try_into().expect("len must be non-negative");
        let raw = env
            .array_with_len(len)
            .expect("env.array_with_len should succeed")
            .as_raw();
        env.return_safe(env.scope().value(raw))
    }

    fn arr_len(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        arr: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> i32 {
        let arr_local = arr
            .try_into_array(env.scope())
            .expect("arrLen expects an array");
        arr_local.len(env).expect("array len should succeed") as i32
    }

    fn arr_push(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        arr: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
        v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> i32 {
        let arr_local = arr
            .try_into_array(env.scope())
            .expect("arrPush expects an array");
        let new_len = arr_local.push(env, v).expect("array push should succeed");
        new_len as i32
    }

    fn arr_set(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        arr: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
        index: i32,
        v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) {
        let arr_local = arr
            .try_into_array(env.scope())
            .expect("arrSet expects an array");

        // Per current project policy: do not enforce a specific OOB/negative strategy here.
        // But we MUST NOT panic across C ABI.
        let Ok(index) = u32::try_from(index) else {
            // Negative index: ignore.
            return;
        };

        // Let QuickJS decide semantics for index > len (may create holes / may throw depending on engine state).
        unsafe {
            let _ = mquickjs_rs::mquickjs_ffi::JS_SetPropertyUint32(
                env.scope().ctx_raw(),
                arr_local.as_raw(),
                index,
                v.as_raw(),
            );
        }
    }

    fn arr_get(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        arr: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
        index: i32,
    ) -> mquickjs_rs::handles::return_safe::ReturnAny {
        let arr_local = arr
            .try_into_array(env.scope())
            .expect("arrGet expects an array");

        let Ok(index) = u32::try_from(index) else {
            return env.return_safe(env.scope().value(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED));
        };

        let raw = unsafe {
            mquickjs_rs::mquickjs_ffi::JS_GetPropertyUint32(
                env.scope().ctx_raw(),
                arr_local.as_raw(),
                index,
            )
        };
        env.return_safe(env.scope().value(raw))
    }
}

pub fn create_test_fn_singleton() -> Box<dyn TestFnSingleton> {
    Box::new(DefaultTestFnSingleton)
}
