use crate::api::TestFnSingleton;

pub struct DefaultTestFnSingleton;

impl TestFnSingleton for DefaultTestFnSingleton {
    fn add_int(&mut self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn echo_any(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        _v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> () {
        unreachable!("any-return must use echo_any_out")
    }

    fn echo_any_out<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        out: &mut dyn for<'hs> FnMut(mquickjs_rs::handles::any::Any<'hs, 'ctx>),
        v: mquickjs_rs::handles::local::Local<'ctx, mquickjs_rs::handles::local::Value>,
    ) -> () {
        out(mquickjs_rs::handles::any::Any::from_value(env.handle(v)))
    }

    fn make_any_string(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        _s: String,
    ) -> () {
        unreachable!("any-return must use make_any_string_out")
    }

    fn make_any_string_out<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        out: &mut dyn for<'hs> FnMut(mquickjs_rs::handles::any::Any<'hs, 'ctx>),
        s: String,
    ) -> () {
        let h = env.str(&s).expect("env.str should succeed");
        out(mquickjs_rs::handles::any::Any::from_value(h))
    }

    fn any_to_string(
        &mut self,
        env: &mut mquickjs_rs::Env<'_>,
        v: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
    ) -> String {
        env.get_string(v).expect("any_to_string expects a string value")
    }

    fn make_array_with_len(
        &mut self,
        _env: &mut mquickjs_rs::Env<'_>,
        _len: i32,
    ) -> () {
        unreachable!("any-return must use make_array_with_len_out")
    }

    fn make_array_with_len_out<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        out: &mut dyn for<'hs> FnMut(mquickjs_rs::handles::any::Any<'hs, 'ctx>),
        len: i32,
    ) -> () {
        let len: u32 = len.try_into().expect("len must be non-negative");
        let h = env
            .array_with_len(len)
            .expect("env.array_with_len should succeed");
        let raw = h.as_raw();
        let v = env.scope().value(raw);
        out(mquickjs_rs::handles::any::Any::from_value(env.handle(v)))
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
        _env: &mut mquickjs_rs::Env<'_>,
        _arr: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>,
        _index: i32,
    ) -> () {
        unreachable!("any-return must use arr_get_out")
    }

    fn arr_get_out<'ctx>(
        &mut self,
        env: &mut mquickjs_rs::Env<'ctx>,
        out: &mut dyn for<'hs> FnMut(mquickjs_rs::handles::any::Any<'hs, 'ctx>),
        arr: mquickjs_rs::handles::local::Local<'ctx, mquickjs_rs::handles::local::Value>,
        index: i32,
    ) -> () {
        let arr_local = arr
            .try_into_array(env.scope())
            .expect("arrGet expects an array");

        let Ok(index) = u32::try_from(index) else {
            // Negative index: return undefined.
            let v = env.scope().value(mquickjs_rs::mquickjs_ffi::JS_UNDEFINED);
            out(mquickjs_rs::handles::any::Any::from_value(env.handle(v)));
            return;
        };

        let raw = unsafe {
            mquickjs_rs::mquickjs_ffi::JS_GetPropertyUint32(env.scope().ctx_raw(), arr_local.as_raw(), index)
        };
        let v = env.scope().value(raw);
        out(mquickjs_rs::handles::any::Any::from_value(env.handle(v)));
    }
}

pub fn create_test_fn_singleton() -> Box<dyn TestFnSingleton> {
    Box::new(DefaultTestFnSingleton)
}
