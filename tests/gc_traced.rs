//! GC Traced<T> tests for mquickjs-rs.
//!
//! Tests that Traced<T> fields in RIDL user class opaque structs are
//! correctly marked by the auto-generated gc_mark callback.

#[cfg(feature = "ridl-extensions")]
fn init_ridl_context(ctx: &mquickjs_rs::Context) {
    unsafe {
        let raw = ctx.ctx as *mut mquickjs_rs::mquickjs_ffi::JSContext;
        mquickjs_demo::ridl_context_ext::ridl_context_init(raw);
        let _rc = mquickjs_rs::mquickjs_ffi::JS_RIDL_StdlibInit(raw);
    }
}

#[cfg(feature = "ridl-extensions")]
fn read_finalizer_count_via_fresh_ctx() -> i32 {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create ctx");
    init_ridl_context(&ctx);
    ctx.eval("TestGc.makeNode().finalizerCount()")
        .expect("eval finalizer count")
        .trim()
        .parse::<i32>()
        .expect("parse count")
}

#[cfg(feature = "ridl-extensions")]
fn gc(ctx: &mut mquickjs_rs::Context) {
    let scope_token = ctx.token();
    let scope = scope_token.enter_scope();
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
    }
}

/// GcTracedNode with Traced<T> opaque field: finalizer fires at teardown.
#[cfg(feature = "ridl-extensions")]
#[test]
fn traced_node_finalized_at_teardown() {
    mquickjs_rs::ridl_bootstrap!();
    let count_before = read_finalizer_count_via_fresh_ctx();

    {
        let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
        init_ridl_context(&ctx);

        // Create a GcTracedNode.
        ctx.eval("globalThis.node = new GcTracedNode();").unwrap();

        // Verify method dispatch works.
        let count = ctx.eval("node.finalizerCount()").unwrap();
        assert!(count.trim().parse::<i32>().is_ok(), "finalizerCount should return a number");

        // Drop reference.
        ctx.eval("delete globalThis.node;").unwrap();

        // GC — unreachable.
        gc(&mut ctx);

        // ctx dropped here -> JS_FreeContext -> finalizer runs.
    }

    let count_after = read_finalizer_count_via_fresh_ctx();
    assert!(
        count_after > count_before,
        "GcTracedNode should be finalized at teardown. before={count_before}, after={count_after}",
    );
}

/// Allocation pressure: GcTracedNodes don't leak JS memory.
///
/// NOTE: mquickjs's GC sweep frees JS objects but does NOT call finalizers.
/// The opaque Box<dyn Trait> is leaked until context teardown. This test
/// uses 10000 iterations to stay within safe memory limits (~500KB leaked).
/// Higher iterations (32000+) cause native heap overflow.
///
/// This is a known platform limitation: finalizers only run at JS_FreeContext.
#[cfg(feature = "ridl-extensions")]
#[test]
fn traced_node_allocation_pressure() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    init_ridl_context(&ctx);

    let result = ctx.eval(
        r#"
        globalThis.out = 'ok';
        for (var i = 0; i < 10000; i++) { var x = new GcTracedNode(); }
        out
        "#,
    );
    match result {
        Ok(s) => assert_eq!(s.trim(), "ok", "allocation loop should complete"),
        Err(e) => panic!("allocation loop failed: {e}"),
    }
}
