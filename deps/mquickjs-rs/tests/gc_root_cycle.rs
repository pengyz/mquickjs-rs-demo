//! GC root 生命周期测试。
//!
//! 这些测试**真实解引用** `Root`，而不是仅断言 "GC 能跑起来"。
//!
//! 关键背景（见 docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md）：
//! `JS_GC` 每次都会压缩堆（移动存活对象）。`Root` 必须能在压缩后
//! 仍指向正确的对象 —— 这由引擎的 `JSGCRef` 机制保证。
//!
//! 复现与差分证明见 `tests/gc_compaction.rs`。

use mquickjs_rs::handles::local::{Local, Value};
use mquickjs_rs::mquickjs_ffi::{self, JSContext, JSGCRef, JSValue};
use std::ffi::CString;

/// 制造大量不可达对象，使后续存活对象在压缩时前移。
const GARBAGE_JS: &str =
    "(function(){var g=[];for(var i=0;i<400;i++){g.push({a:i,b:i,c:i});}return 0;})()";

fn gc(scope: &mquickjs_rs::handles::scope::Scope<'_>) {
    unsafe { mquickjs_ffi::JS_GC(scope.ctx_raw()) };
}

/// 解码整数 JSValue（`JS_TAG_INT = 0`，值在 `v >> 1`）。
fn decode_int(v: JSValue) -> Option<i64> {
    if (v & 1) == 0 {
        Some((v as i64) >> 1)
    } else {
        None
    }
}

fn get_int_prop(ctx: *mut JSContext, obj: JSValue, key: &str) -> Option<i64> {
    let name = CString::new(key).unwrap();
    let v = unsafe { mquickjs_ffi::JS_GetPropertyStr(ctx, obj, name.as_ptr()) };
    decode_int(v)
}

/// `Root` 在 GC（含堆压缩）后仍指向**正确对象**。
#[test]
fn gc_root_keeps_value_alive_and_relocated_across_gc() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");

    // 先制造垃圾，再分配目标对象（目标位于垃圾之后 → 压缩时会前移）
    ctx.eval_jsvalue(GARBAGE_JS).expect("eval garbage");
    let raw = ctx
        .eval_jsvalue("({marker: 4242})")
        .expect("eval object");

    let token = ctx.token();
    let scope = token.enter_scope();

    let local: Local<'_, Value> = scope.value(raw);
    let root = mquickjs_rs::Root::new(&scope, local);

    let before = root.as_raw();
    assert_eq!(
        get_int_prop(scope.ctx_raw(), before, "marker"),
        Some(4242),
        "GC 前应能通过 Root 读到 marker"
    );

    // GC：垃圾被回收，目标对象被压缩前移
    gc(&scope);

    // 关键断言：压缩后仍能通过 Root 读到**正确的** marker。
    // 若 Root 未随压缩重定位，这里会读到错误数据或崩溃。
    let after = root.as_raw();
    assert_eq!(
        get_int_prop(scope.ctx_raw(), after, "marker"),
        Some(4242),
        "GC（含压缩）后仍应通过 Root 读到正确的 marker"
    );

    println!(
        "root relocation: before={:#x} after={:#x} moved={}",
        before,
        after,
        before != after
    );
}

/// `Root` 的地址与引擎权威值（`JSGCRef`）保持一致。
#[test]
fn gc_root_agrees_with_engine_authoritative_value() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    ctx.eval_jsvalue(GARBAGE_JS).expect("eval garbage");
    let raw = ctx.eval_jsvalue("({marker: 7})").expect("eval object");

    let token = ctx.token();
    let scope = token.enter_scope();

    let local: Local<'_, Value> = scope.value(raw);
    let root = mquickjs_rs::Root::new(&scope, local);

    // 引擎权威基准：JSGCRef 一定被正确重定位
    let mut gc_ref = Box::new(JSGCRef {
        val: mquickjs_ffi::JS_UNDEFINED,
        prev: std::ptr::null_mut(),
    });
    let slot = unsafe { mquickjs_ffi::JS_AddGCRef(scope.ctx_raw(), &mut *gc_ref) };
    unsafe { *slot = raw };

    gc(&scope);

    let authoritative = unsafe { *slot };
    assert_eq!(
        root.as_raw(),
        authoritative,
        "Root 应与引擎 JSGCRef 指向同一地址（即 Root 确实被重定位）"
    );

    unsafe { mquickjs_ffi::JS_DeleteGCRef(scope.ctx_raw(), &mut *gc_ref) };
}

/// 释放 `Root` 后，对象可以被回收（Root 不再保活它）。
#[test]
fn gc_root_removed_allows_collection_of_cycle() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 两个对象互相引用，形成环
    let b = ctx.eval_jsvalue("({name: 11})").expect("eval b");
    let a = ctx.eval_jsvalue("({name: 22})").expect("eval a");

    let b_local: Local<'_, Value> = scope.value(b);
    let b_root = mquickjs_rs::Root::new(&scope, b_local);

    let a_local: Local<'_, Value> = scope.value(a);
    let a_root = mquickjs_rs::Root::new(&scope, a_local);

    unsafe {
        let name = CString::new("back").unwrap();
        mquickjs_ffi::JS_SetPropertyStr(
            scope.ctx_raw(),
            b_root.as_raw(),
            name.as_ptr(),
            a_root.as_raw(),
        );
    }

    // 两个 Root 都在：GC 后仍可访问
    gc(&scope);
    assert_eq!(
        get_int_prop(scope.ctx_raw(), b_root.as_raw(), "name"),
        Some(11),
        "Root 保活期间应能读到 b.name"
    );

    // 释放最后一个 Root：环只存在于 JS 堆内 → 可被回收
    drop(a_root);
    drop(b_root);

    // 仅验证 GC 能安全完成（对象已无外部 root）
    gc(&scope);
    gc(&scope);
}