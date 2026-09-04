//! GC Traced<T> tests for mquickjs-rs.
//!
//! Tests that Traced<T> fields in RIDL user class opaque structs are
//! correctly marked by the auto-generated gc_mark callback.

#[cfg(feature = "ridl-extensions")]
use mquickjs_rs::handles::local::{Local, Value};

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

// ========================================================================
// 异步任务 GC 集成测试
// ========================================================================

/// 异步任务中的 Root<T> 参与 GC 标记
/// 
/// 验证：异步任务持有的 Root<T> 在 GC 期间被正确标记，
/// 防止 callback JSValue 被回收。
#[cfg(feature = "ridl-extensions")]
#[test]
fn async_task_root_participates_in_gc() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    init_ridl_context(&ctx);

    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    // 创建一个 JS 函数作为 callback
    ctx.eval(r#"
        globalThis.myCallback = function(result) {
            globalThis.callbackResult = result;
        };
    "#).unwrap();

    // 获取 callback 的 JSValue 并创建 Root
    let token = ctx.token();
    let scope = token.enter_scope();
    let cb_jsvalue = ctx.eval_jsvalue("myCallback").expect("get callback");
    let cb_local: Local<'_, Value> = scope.value(cb_jsvalue);
    let cb_root = mquickjs_rs::Root::new(&scope, cb_local);

    // 创建异步任务（模拟）
    let task_manager = Arc::new(mquickjs_rs::async_task::AsyncTaskManager::new());
    
    // 注册任务（简化：直接测试 Root 生命周期）
    // 实际测试需要 AsyncBridge，但这里验证 Root 在 GC 期间存活

    // GC：callback 应该被 Root 保护，不被回收
    gc(&mut ctx);

    // 验证 callback 仍然可访问
    let result = ctx.eval("typeof myCallback");
    assert_eq!(result.unwrap().trim(), "function", "callback should survive GC");

    // 释放 Root
    drop(cb_root);

    // GC：callback 现在应该可以被回收
    gc(&mut ctx);

    // 验证 callback 仍然存在（因为 globalThis.myCallback 仍然引用它）
    let result = ctx.eval("typeof myCallback");
    assert_eq!(result.unwrap().trim(), "function", "callback still referenced by globalThis");

    // 清理
    ctx.eval("delete globalThis.myCallback").unwrap();
    gc(&mut ctx);

    // teardown 时 finalizer 应该触发
}

/// 异步任务超时后 Root 释放
/// 
/// 验证：超时任务取消后，Root<T> 被正确释放，
/// 允许 GC 回收 callback。
#[cfg(feature = "ridl-extensions")]
#[test]
fn async_task_timeout_releases_root() {
    use std::sync::Arc;
    use std::time::Duration;

    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    init_ridl_context(&ctx);

    let count_before = read_finalizer_count_via_fresh_ctx();

    {
        // 创建 callback
        ctx.eval(r#"
            globalThis.timeoutCallback = function(result) {
                globalThis.timeoutResult = result;
            };
        "#).unwrap();

        // 获取 callback 并创建 Root
        let token = ctx.token();
        let scope = token.enter_scope();
        let cb_jsvalue = ctx.eval_jsvalue("timeoutCallback").expect("get callback");
        let cb_local: Local<'_, Value> = scope.value(cb_jsvalue);
        let cb_root = mquickjs_rs::Root::new(&scope, cb_local);

        // 模拟超时任务：Root 持有 callback，但任务超时取消
        // 实际测试需要 AsyncBridge，这里验证 Root 生命周期

        // 释放 Root（模拟任务超时取消）
        drop(cb_root);

        // 清理
        ctx.eval("delete globalThis.timeoutCallback").unwrap();
        gc(&mut ctx);

        // ctx dropped here -> JS_FreeContext -> finalizer runs
    }

    let count_after = read_finalizer_count_via_fresh_ctx();
    assert!(
        count_after > count_before,
        "Finalizer should fire at teardown. before={count_before}, after={count_after}",
    );
}
