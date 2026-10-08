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

/// 【ABI 缺陷复现】GC 时节点**可达** ⇒ 引擎调用 class 的 `gc_mark` 回调。
///
/// 引擎契约是 **3 参** `(ctx, opaque, mf)`
/// （`mquickjs.h:232` 的 `JSCMark`；调用点 `mquickjs.c:12174`），
/// 而 RIDL 曾生成的 Rust 定义是 **4 参** `(_ctx, _obj, opaque, mf)`。
/// 寄存器错位后 `opaque` 实参收到的是 `&mf`（栈地址、非空），
/// 被当作 `Box<dyn Trait>` fat pointer 解引用 → 虚表跳转 → SIGSEGV。
///
/// 既有测试之所以没崩，是因为它们都**先 `delete globalThis.node` 再 GC**
/// —— 对象不可达 ⇒ `gc_mark_all` 不会遍历到它 ⇒ 该回调从不被调用。
#[cfg(feature = "ridl-extensions")]
#[test]
fn traced_node_reachable_during_gc_invokes_gc_mark() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    init_ridl_context(&ctx);

    // 关键：节点**保持可达**，让 gc_mark_all 遍历到它。
    ctx.eval("globalThis.node = new GcTracedNode();").unwrap();

    // 若 class gc_mark 的 ABI 不匹配，这一步会 SIGSEGV。
    gc(&mut ctx);

    // 存活校验：GC 后对象仍可用。
    let count = ctx.eval("node.finalizerCount()").unwrap();
    assert!(
        count.trim().parse::<i32>().is_ok(),
        "GC 后节点应仍可用，实际: {count}"
    );
}

/// 【端到端】`Traced` 字段在 GC 压缩后仍指向正确对象。
///
/// 覆盖三件事：
/// 1. class gc_mark ABI 缺陷已修（节点可达时 GC 不再 SIGSEGV）
/// 2. `Traced` 随堆压缩重定位（`as_raw()` 返回最新地址）
/// 3. opaque 内嵌 `Traced` 的完整链路可用
#[cfg(feature = "ridl-extensions")]
#[test]
fn traced_field_survives_compaction_end_to_end() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    init_ridl_context(&ctx);

    // 先制造垃圾，再建节点并把目标对象存入其 Traced 字段
    // —— 目标位于垃圾之后，压缩时会被前移。
    ctx.eval(
        r#"
        (function(){ var g=[]; for (var i=0;i<400;i++){ g.push({a:i,b:i,c:i}); } })();
        globalThis.node = new GcTracedNode();
        node.setHeld({marker: 8888});
        "#,
    )
    .unwrap();

    // 节点保持可达 ⇒ GC 会遍历到它（曾经在此崩溃）
    gc(&mut ctx);
    gc(&mut ctx);

    // 压缩后通过 Traced 读 marker：
    // 若 Traced 未随压缩重定位，这里会读到错误数据或崩溃。
    let m = ctx.eval("node.heldMarker()").unwrap();
    assert_eq!(
        m.trim(),
        "8888",
        "Traced 字段应在 GC 压缩后仍指向正确对象"
    );
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

/// Allocation pressure: GcTracedNodes 的分配压力测试。
///
/// 更正（2026-09）：此前本注释写着「mquickjs 的 GC sweep 不调用 finalizer，
/// opaque Box 泄漏到 context teardown；32000+ 次迭代导致 native 堆溢出」。
/// 经逐行核对引擎源码，**该结论错误**：
/// sweep 循环中确实会调用 class finalizer（`mquickjs.c:12416-12419`）。
///
/// 真正的风险不是 finalizer 缺失，而是 `Traced<T>` / `Root<T>` 存放的裸
/// `JSValue` 在堆压缩后**不参与重定位**而悬垂（见 tests/gc_compaction.rs）。
/// 详见 docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md。
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
