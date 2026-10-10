//! CallbackRegistry —— 同步 many-shot 回调桥单元测试（设计切片 1）。
//!
//! 设计文档：`docs/superpowers/specs/2026-10-10-callback-bridge-design.md`
//! §3 切片 1。写法跟随 `tests/gc_compaction.rs` / `tests/gc_root_cycle.rs`。
//!
//! 场景清单（任务规定）：
//! 1. 注册 → 显式 JS_GC（含压缩）→ invoke 仍触发 JS 副作用
//! 2. many-shot：同一 handle ≥2 次 invoke 均成功
//! 3. unregister → JS_GC → 函数可被回收（槽位释放 + GC 安全）
//! 4. invoke 不存在的 handle → Err(InvalidHandle)，ctx 无残留异常
//! 5. JS 函数体 throw → Err(JsException)，pending exception 已清除
//! 6. Context 销毁 → JSGCRef 全释放（teardown 安全；存活 token 得到
//!    ContextDropped 而非 UB）
//!
//! # 关于"函数被回收"的验证方式
//!
//! mquickjs stdlib 是最小集，没有 WeakRef / FinalizationRegistry，无法
//! 从 JS 侧直接观测回收。按 `gc_root_cycle.rs` 的先例采用间接证据：
//!
//! - unregister 后槽位**立即释放**（新注册复用同一槽位句柄）；
//! - 注销后多次 GC（含压缩）安全完成，且**仍注册的**回调不受影响；
//! - 被测函数由 eval 表达式直接产生、不落全局 —— registry 是唯一持有者，
//!   槽位释放后它不可达，必然可被 mark-sweep 回收。

use mquickjs_rs::context::ContextToken;
use mquickjs_rs::handles::local::{Function, Local, Value};
use mquickjs_rs::mquickjs_ffi::{self, JSContext};
use mquickjs_rs::{CallbackError, CallbackHandle, Context};

/// 制造大量不可达对象，使后续存活对象在压缩时前移（同 gc_root_cycle.rs）。
const GARBAGE_JS: &str =
    "(function(){var g=[];for(var i=0;i<400;i++){g.push({a:i,b:i,c:i});}return 0;})()";

fn gc(ctx: *mut JSContext) {
    unsafe { mquickjs_ffi::JS_GC(ctx) };
}

/// 解码整数 JSValue（`JS_TAG_INT = 0`，值在 `v >> 1`）。
fn decode_int(v: mquickjs_rs::mquickjs_ffi::JSValue) -> Option<i64> {
    if (v & 1) == 0 {
        Some((v as i64) >> 1)
    } else {
        None
    }
}

/// eval 一个函数表达式并转成 `Local<Function>`。
///
/// 函数**不落全局**：注册后 registry 是它的唯一持有者，
/// 这是场景 3（回收）成立的前提。
fn eval_function<'s>(
    ctx: &mut Context,
    scope: &mquickjs_rs::Scope<'s>,
    src: &str,
) -> Local<'s, Function> {
    let raw = ctx.eval_jsvalue(src).expect("eval function source");
    let lval: Local<'_, Value> = scope.value(raw);
    lval.try_into_function(scope).expect("eval produced a function")
}

fn eval_i64(ctx: &mut Context, expr: &str) -> i64 {
    let raw = ctx.eval_jsvalue(expr).expect("eval int expression");
    decode_int(raw).unwrap_or_else(|| panic!("expected int from {expr}"))
}

/// 解码字符串结果（对齐 Context::eval 的 JS_ToCString 手法）。
fn eval_string(ctx: &mut Context, expr: &str) -> String {
    ctx.eval(expr).expect("eval string expression")
}

/// 场景 1：注册 → 显式 JS_GC（含压缩）→ invoke 仍触发 JS 副作用。
#[test]
fn registered_callback_survives_gc_compaction_and_invoke_runs_it() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");
    ctx.eval_jsvalue("var __hits = 0;").expect("init counter");

    let token = ctx.token();
    let scope = token.enter_scope();

    // 先制造垃圾，再分配目标函数（对齐 gc_root_cycle 的分配顺序：
    // 目标位于垃圾之后 → 压缩时必然前移）。
    ctx.eval_jsvalue(GARBAGE_JS).expect("make garbage");

    let f = eval_function(
        &mut ctx,
        &scope,
        "(function (ev) { __hits = __hits + (ev|0) + 1; })",
    );
    let handle = ctx.callbacks().register(&scope, f);

    // 显式 JS_GC：mquickjs 每次 JS_GC 都压缩堆（memmove 前移存活对象），
    // 位于垃圾之后的函数对象必然被移动 —— JSGCRef 槽位必须已被引擎
    // 重定位，invoke 才能调用到正确的函数。
    gc(ctx.ctx);
    gc(ctx.ctx);

    // invoke 仍触发 JS 副作用，且参数正确传递（4 + 1 = 5）。
    let arg = unsafe { mquickjs_ffi::JS_NewInt32(ctx.ctx, 4) };
    ctx.callbacks()
        .invoke(handle, &[arg])
        .expect("invoke after GC (with compaction)");

    assert_eq!(
        eval_i64(&mut ctx, "__hits"),
        5,
        "回调应在 GC（含压缩）后仍执行，且收到参数 4"
    );
}

/// 场景 2：many-shot —— 同一 handle ≥2 次 invoke 均成功。
#[test]
fn same_handle_supports_many_shot_invocations() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");
    ctx.eval_jsvalue("var __sum = 0;").expect("init sum");

    let token = ctx.token();
    let scope = token.enter_scope();

    let f = eval_function(&mut ctx, &scope, "(function (ev) { __sum = __sum + (ev|0); })");
    let handle = ctx.callbacks().register(&scope, f);

    for (arg, expected_partial) in [(2i64, 2), (3, 5), (-1, 4)] {
        let js_arg = unsafe { mquickjs_ffi::JS_NewInt32(ctx.ctx, arg as i32) };
        ctx.callbacks()
            .invoke(handle, &[js_arg])
            .unwrap_or_else(|e| panic!("many-shot invoke #{arg} failed: {e:?}"));
        assert_eq!(
            eval_i64(&mut ctx, "__sum"),
            expected_partial,
            "第 {arg} 次触发后累加值错误"
        );
    }
}

/// 场景 3：unregister → JS_GC → 函数可被回收（槽位立即释放 + GC 安全）。
///
/// 回收本身按 gc_root_cycle.rs 先例做间接验证（mquickjs 无 WeakRef），
/// 见本文件头部说明。
#[test]
fn unregister_releases_slot_and_function_becomes_collectable() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");
    ctx.eval_jsvalue("var __kept = 0;").expect("init counter");

    let token = ctx.token();
    let scope = token.enter_scope();

    // f1 / f2 不落全局，注册后 registry 是唯一持有者。
    let f1 = eval_function(&mut ctx, &scope, "(function () { __kept = __kept + 1; })");
    let h1 = ctx.callbacks().register(&scope, f1);
    let f2 = eval_function(&mut ctx, &scope, "(function () { __kept = __kept + 10; })");
    let h2 = ctx.callbacks().register(&scope, f2);

    ctx.callbacks().invoke(h1, &[]).expect("invoke f1");
    ctx.callbacks().invoke(h2, &[]).expect("invoke f2");
    assert_eq!(eval_i64(&mut ctx, "__kept"), 11);

    // 注销 f1；重复注销应报告"槽位本就为空"。
    assert!(ctx.callbacks().unregister(h1), "首次 unregister 应成功");
    assert!(!ctx.callbacks().unregister(h1), "重复 unregister 应返回 false");

    // 双重 GC（含压缩）：f1 不可达、被回收；f2 仍被 registry 保活。
    ctx.eval_jsvalue(GARBAGE_JS).expect("make garbage");
    gc(ctx.ctx);
    gc(ctx.ctx);

    // f2（仍注册）在 GC 后正常 —— registry 与 JSGCRef 链未损坏。
    ctx.callbacks().invoke(h2, &[]).expect("invoke f2 after GC");
    assert_eq!(eval_i64(&mut ctx, "__kept"), 21);

    // 已注销的 handle → InvalidHandle（f1 已不被 registry 持有）。
    let err = ctx.callbacks().invoke(h1, &[]).unwrap_err();
    assert!(
        matches!(err, CallbackError::InvalidHandle),
        "已注销句柄应报 InvalidHandle，实际 {err:?}"
    );

    // 槽位复用：新注册应拿到 h1 释放的槽位（句柄相同），
    // 证明对应 JSGCRef 已从引擎链表摘除。
    let f3 = eval_function(&mut ctx, &scope, "(function () { __kept = __kept + 100; })");
    let h3: CallbackHandle = ctx.callbacks().register(&scope, f3);
    assert_eq!(h3, h1, "新注册应复用已释放的槽位");

    ctx.callbacks().invoke(h3, &[]).expect("invoke f3");
    assert_eq!(eval_i64(&mut ctx, "__kept"), 121);

    // 收尾：注销全部后 GC 安全完成。
    assert!(ctx.callbacks().unregister(h2));
    assert!(ctx.callbacks().unregister(h3));
    gc(ctx.ctx);
    gc(ctx.ctx);
}

/// 场景 4：invoke 不存在的 handle → Err(InvalidHandle)，ctx 无残留异常。
#[test]
fn invoke_with_unknown_handle_is_invalid_and_leaves_no_exception() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");

    let token = ctx.token();
    let scope = token.enter_scope();

    let f = eval_function(&mut ctx, &scope, "(function () { })");
    let stale = ctx.callbacks().register(&scope, f);
    assert!(ctx.callbacks().unregister(stale));

    let err = ctx.callbacks().invoke(stale, &[]).unwrap_err();
    assert!(
        matches!(err, CallbackError::InvalidHandle),
        "应报 InvalidHandle，实际 {err:?}"
    );

    // 关键：无效句柄路径**不得**在 ctx 留下 pending exception，
    // 否则下一次 eval 会莫名失败。
    unsafe {
        assert_eq!(
            mquickjs_ffi::JS_HasException(ctx.ctx),
            0,
            "InvalidHandle 路径不得残留 pending exception"
        );
    }
    assert_eq!(eval_string(&mut ctx, "1 + 1"), "2", "ctx 应可继续正常工作");
}

/// 场景 5：JS 函数体 throw → Err(JsException)，pending exception 已清除。
#[test]
fn throwing_callback_returns_js_exception_and_clears_pending_exception() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");

    let token = ctx.token();
    let scope = token.enter_scope();

    let f = eval_function(&mut ctx, &scope, "(function () { throw 'boom42'; })");
    let handle = ctx.callbacks().register(&scope, f);

    let err = ctx.callbacks().invoke(handle, &[]).unwrap_err();
    match &err {
        CallbackError::JsException(msg) => {
            assert!(msg.contains("boom42"), "异常消息应包含 'boom42'，实际 {msg:?}");
        }
        other => panic!("应报 JsException，实际 {other:?}"),
    }

    // JS_GetException 取走并清除 pending exception（mquickjs.c:2139）：
    // 异常绝不残留，否则 ctx 后续所有操作都会"莫名失败"。
    unsafe {
        assert_eq!(
            mquickjs_ffi::JS_HasException(ctx.ctx),
            0,
            "JsException 路径必须清除 pending exception"
        );
    }

    // ctx 可继续正常工作；同一 handle（many-shot）可再次触发。
    assert_eq!(eval_string(&mut ctx, "40 + 2"), "42");
    let again = ctx.callbacks().invoke(handle, &[]).unwrap_err();
    assert!(matches!(again, CallbackError::JsException(_)));
    unsafe {
        assert_eq!(mquickjs_ffi::JS_HasException(ctx.ctx), 0);
    }
}

/// 审查修复回归：异常对象在 stringify 期间用户 toString/valueOf 再抛
/// （嵌套 throw）→ invoke 返回 Err 且 ctx **无任何残留异常**。
///
/// 引擎事实：`JS_ToCString` → `JS_ToString` → `JS_ToPrimitive` 会取并
/// 调用**用户定义的** toString/valueOf（mquickjs.c:4285-4310）；其再抛时
/// 新异常留在 ctx、JS_ToCString 返回 null。invoke 的 null 分支必须清扫
/// 该嵌套异常，"返回 Err ⇒ ctx 无残留"才对所有异常形状成立。
#[test]
fn nested_throw_during_stringification_leaves_no_pending_exception() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");

    let token = ctx.token();
    let scope = token.enter_scope();

    let f = eval_function(
        &mut ctx,
        &scope,
        "(function () { throw { toString: function () { throw 1; } }; })",
    );
    let handle = ctx.callbacks().register(&scope, f);

    let err = ctx.callbacks().invoke(handle, &[]).unwrap_err();
    match &err {
        CallbackError::JsException(msg) => {
            assert!(
                msg.contains("stringifying"),
                "应报告 stringify 期间再抛，实际 {msg:?}"
            );
        }
        other => panic!("应报 JsException，实际 {other:?}"),
    }

    // 关键断言：嵌套异常已被清扫，ctx 干净。
    unsafe {
        assert_eq!(
            mquickjs_ffi::JS_HasException(ctx.ctx),
            0,
            "嵌套 throw 后 ctx 不得残留 pending exception"
        );
    }

    // ctx 可继续正常工作；同一 handle 再次触发路径稳定（many-shot）。
    assert_eq!(eval_string(&mut ctx, "'still' + ' working'"), "still working");
    let again = ctx.callbacks().invoke(handle, &[]).unwrap_err();
    assert!(matches!(again, CallbackError::JsException(_)));
    unsafe {
        assert_eq!(mquickjs_ffi::JS_HasException(ctx.ctx), 0);
    }
}

/// 场景 6：Context 销毁 → JSGCRef 全释放（携带未注销句柄的 teardown 安全）。
///
/// - 槽位（`Box<JSGCRef>`）随 `ContextInner` 静默 drop —— 与 `RootsRegistry`
///   一致，绝不调用引擎 API（此时引擎 gc_ref 链已随 context 内存消失）；
/// - 存活的 `ContextToken` 再访问得到 `ContextDropped`（alive 护栏），
///   而不是触碰悬垂 ctx 指针。
#[test]
fn context_drop_with_live_callbacks_releases_all_refs_safely() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();

    let (h0, h1) = {
        let scope = token.enter_scope();
        let f0 = eval_function(&mut ctx, &scope, "(function () { })");
        let h0 = ctx.callbacks().register(&scope, f0);
        let f1 = eval_function(&mut ctx, &scope, "(function () { })");
        let h1 = ctx.callbacks().register(&scope, f1);

        // 注册即有效
        ctx.callbacks().invoke(h0, &[]).expect("invoke before drop");
        (h0, h1)
    };

    // 携带未注销句柄直接销毁 Context。
    drop(ctx);

    // alive 护栏：经存活 token 访问得到类型化错误，绝非 UB。
    let err = token.callbacks().invoke(h0, &[]).unwrap_err();
    assert!(
        matches!(err, CallbackError::ContextDropped),
        "销毁后 invoke 应报 ContextDropped，实际 {err:?}"
    );
    assert!(
        !token.callbacks().unregister(h1),
        "销毁后 unregister 应安全地报告无槽位"
    );

    // 引擎/进程无残留损坏：全新 context 正常工作。
    let mut ctx2 = Context::new(1024 * 1024).expect("fresh context after drop");
    assert_eq!(eval_string(&mut ctx2, "'alive' + '!'"), "alive!");
}

/// 存活 token 对已销毁 context 的 register 是编程错误 → panic（而非 UB）。
#[test]
#[should_panic(expected = "context dropped")]
fn register_through_dead_context_panics_instead_of_ub() {
    let mut ctx = Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();
    let scope = token.enter_scope();

    let f = eval_function(&mut ctx, &scope, "(function () { })");

    drop(ctx);

    // register 在触碰引擎之前先做 alive 断言 → panic，不会解引用悬垂 ctx。
    let _ = ctx_is_gone_register(&token, &scope, f);
}

fn ctx_is_gone_register<'s>(
    token: &ContextToken,
    scope: &mquickjs_rs::Scope<'s>,
    f: Local<'s, Function>,
) -> CallbackHandle {
    token.callbacks().register(scope, f)
}
