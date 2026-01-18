#[cfg(test)]
mod tests {
    use crate::handles::global::Global;
    use crate::handles::local::Value;

    #[test]
    fn nested_enter_guards_stack_order() {
        let ctx = crate::context::Context::new(1024 * 1024).unwrap();
        let h = ctx.token();
        let _s1 = crate::handles::scope::Scope::from_handle(&h);
        {
            let _s2 = crate::handles::scope::Scope::from_handle(&h);
            let _ = (_s1.ctx_raw(), _s2.ctx_raw());
        }
    }

    #[test]
    fn global_drop_after_context_panics() {
        // This is a strict rule: Global must be dropped before Context.
        let leaked = {
            let ctx = crate::context::Context::new(1024 * 1024).unwrap();
            let h = ctx.token();
            let scope = crate::handles::scope::Scope::from_handle(&h);
            let v = scope.value(crate::mquickjs_ffi::JS_UNDEFINED);
            let g: Global<Value> = Global::new(&scope, v);
            std::mem::forget(scope);
            g
        };

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(leaked)));
        assert!(res.is_err());
    }
}
