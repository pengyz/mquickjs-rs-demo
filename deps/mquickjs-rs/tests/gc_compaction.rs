//! GC 压缩与 Root 重定位测试
//!
//! 背景（见 docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md）：
//!
//! - `JS_GC` → `JS_GC2` → `gc_mark_all` + `gc_compact_heap`，
//!   **每一次 JS_GC 都会压缩堆（memmove 移动存活对象）**。
//! - `gc_compact_heap` 的重定位清单只包含：context 内建字段、
//!   `string_pos_cache`、JS 值栈、**`JSGCRef` 链**、`parse_state`。
//! - 通过 `JS_SetContextGCMark` 注册的自建 root registry **只在 mark 阶段被调用**，
//!   其持有的 `JSValue` **不参与重定位**。
//!
//! 因此：自建 `RootsRegistry` 中的 `JSValue` 在压缩后可能悬垂。
//! 本文件用「`JSGCRef`（引擎重定位）vs `Root`（自建，不重定位）」
//! 的差分对比来证明该缺陷。

use mquickjs_rs::handles::local::{Local, Value};
use mquickjs_rs::mquickjs_ffi::{self, JSContext, JSGCRef, JSValue};
use std::ffi::CString;

/// 在堆上制造大量不可达对象，随后释放，使后续存活对象在压缩时前移。
const GARBAGE_JS: &str = "(function(){var g=[];for(var i=0;i<400;i++){g.push({a:i,b:i,c:i});}return 0;})()";

fn gc(ctx: *mut JSContext) {
    unsafe { mquickjs_ffi::JS_GC(ctx) };
}

/// 解码 JSValue 中的整数。
///
/// mquickjs 的编码（`mquickjs.h:56-78`）：
/// - `JS_TAG_INT = 0` → **整数不是 special tag**，判定为 `(v & 1) == 0`，值为 `v >> 1`
/// - `JS_TAG_PTR = 1` → `(v & 3) == 1`
/// - `JS_TAG_SPECIAL = 3` → `(v & 3) == 3`，低 5 位是 special tag
fn decode_int(v: JSValue) -> Option<i64> {
    if (v & 1) == 0 {
        Some((v as i64) >> 1)
    } else {
        None
    }
}

fn get_marker(ctx: *mut JSContext, obj: JSValue) -> Option<i64> {
    let name = CString::new("marker").unwrap();
    let v = unsafe { mquickjs_ffi::JS_GetPropertyStr(ctx, obj, name.as_ptr()) };
    decode_int(v)
}

/// 证明：JS_GC 会移动堆上对象（即压缩真实发生）。
///
/// 用一个 `JSGCRef` 持有对象——引擎会在压缩时更新它。
/// 若 GC 后 `JSGCRef.val` 变化，说明对象确实被移动了。
#[test]
fn gc_compaction_actually_moves_objects() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let handle = ctx.token();
    let _g = handle.enter_current();
    let raw_ctx = handle.ctx;

    // 先制造垃圾，再分配目标对象（目标位于垃圾之后）
    ctx.eval_jsvalue(GARBAGE_JS).expect("eval garbage");
    let target = ctx
        .eval_jsvalue("({marker: 12345})")
        .expect("eval target object");

    // 用引擎认可的 JSGCRef 持有目标（压缩安全）
    let mut gc_ref = Box::new(JSGCRef {
        val: mquickjs_ffi::JS_UNDEFINED,
        prev: std::ptr::null_mut(),
    });
    let slot = unsafe { mquickjs_ffi::JS_AddGCRef(raw_ctx, &mut *gc_ref) };
    assert!(!slot.is_null(), "JS_AddGCRef returned null");
    unsafe { *slot = target };

    let before = unsafe { *slot };
    assert_eq!(get_marker(raw_ctx, before), Some(12345), "GC 前应能读到 marker");

    // 触发 GC：垃圾被回收 → 压缩把目标前移
    gc(raw_ctx);

    let after = unsafe { *slot };
    assert_eq!(
        get_marker(raw_ctx, after),
        Some(12345),
        "GC 后通过 JSGCRef 应仍能读到 marker（JSGCRef 是压缩安全的）"
    );

    println!(
        "compaction probe: before={:#x} after={:#x} moved={}",
        before,
        after,
        before != after
    );

    unsafe { mquickjs_ffi::JS_DeleteGCRef(raw_ctx, &mut *gc_ref) };
}

/// 核心缺陷测试：`Root<T>` 持有的 `JSValue` 在压缩后不更新。
///
/// 差分对比：
/// - `JSGCRef.val` → 引擎重定位 → 指向移动后的真实对象
/// - `Root::as_raw()` → 自建 registry，不重定位 → 仍指向旧地址
///
/// 若二者在 GC 后不一致，则 `Root` 已悬垂。
#[test]
fn gc_compaction_does_not_relocate_root_registry() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let handle = ctx.token();
    let raw_ctx = handle.ctx;

    // 制造垃圾 → 目标对象
    ctx.eval_jsvalue(GARBAGE_JS).expect("eval garbage");
    let target = ctx
        .eval_jsvalue("({marker: 12345})")
        .expect("eval target object");

    // 用 Root 持有（走自建 RootsRegistry）
    let scope = handle.enter_scope();
    let local: Local<'_, Value> = scope.value(target);
    let root = mquickjs_rs::Root::new(&scope, local);
    let root_before = root.as_raw();

    // 用 JSGCRef 持有同一对象（压缩安全基准）
    let mut gc_ref = Box::new(JSGCRef {
        val: mquickjs_ffi::JS_UNDEFINED,
        prev: std::ptr::null_mut(),
    });
    let slot = unsafe { mquickjs_ffi::JS_AddGCRef(raw_ctx, &mut *gc_ref) };
    unsafe { *slot = target };

    assert_eq!(get_marker(raw_ctx, root_before), Some(12345));

    // 触发 GC（压缩）
    gc(raw_ctx);

    let authoritative = unsafe { *slot };

    // 基准：JSGCRef 一定正确
    assert_eq!(
        get_marker(raw_ctx, authoritative),
        Some(12345),
        "JSGCRef 基准值应正确"
    );

    // Root 是否被重定位？
    let root_after = root.as_raw();
    println!(
        "root={:#x} relocated={:#x} same_as_authoritative={}",
        root_after,
        authoritative,
        root_after == authoritative
    );

    assert_eq!(
        root_after, authoritative,
        "缺陷：Root 持有的 JSValue 未随压缩重定位\n\
         对象实际已移动到 {:#x}，但 Root 仍持有 {:#x}\n\
         此后通过 Root 解引用将访问已移动/已释放的内存",
        authoritative, root_after
    );

    unsafe { mquickjs_ffi::JS_DeleteGCRef(raw_ctx, &mut *gc_ref) };
}