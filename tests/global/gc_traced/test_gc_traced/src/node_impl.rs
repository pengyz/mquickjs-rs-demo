use std::ffi::CString;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::api::GcTracedNodeClass;
use mquickjs_rs::handles::local::{Local, Value};
use mquickjs_rs::mquickjs_ffi;

static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

/// RIDL class `GcTracedNode`：opaque 里持有 `Traced<Value>` 字段。
///
/// `Traced` 基于引擎的 `JSGCRef`（见 `mquickjs_rs::Traced` 文档），
/// 该链在 GC 的 mark 与重定位两个阶段都被引擎扫描，
/// 因此压缩后仍指向正确的对象。
///
/// 注意：本类型**不再**实现任何 `gc_mark` 覆写 ——
/// class 级 gc_mark 回调已整体废弃（它曾带有 4 参 vs 3 参的致命 ABI 缺陷）。
pub struct DefaultGcTracedNode {
    pub held: Option<mquickjs_rs::Traced<Value>>,
}

impl DefaultGcTracedNode {
    pub fn new() -> Self {
        Self { held: None }
    }
}

impl Drop for DefaultGcTracedNode {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

impl GcTracedNodeClass for DefaultGcTracedNode {
    fn finalizer_count(&mut self) -> i32 {
        FINALIZER_COUNT.load(Ordering::SeqCst)
    }

    /// 把传入的 JS 值存入 opaque 的 `Traced` 字段。
    ///
    /// 参数含 `any` ⇒ `needs_scope` ⇒ 拿到 `Env`，可从中取 `Scope`
    /// 用于注册压缩安全的 GC 句柄。
    fn set_held<'ctx>(&mut self, env: &mut mquickjs_rs::Env<'ctx>, v: Local<'ctx, Value>) -> i32 {
        self.held = Some(mquickjs_rs::Traced::new(env.scope(), v));
        1
    }

    /// 读取 `held` 所指对象的 `marker` 属性。
    ///
    /// 该读取发生在 GC（含堆压缩）之后 —— 若 `Traced` 未随压缩重定位，
    /// 这里会读到错误数据或崩溃。
    fn held_marker(&mut self) -> i32 {
        let Some(h) = mquickjs_rs::context::ContextToken::current() else {
            return -1;
        };
        let Some(held) = self.held.as_ref() else {
            return -2;
        };
        let name = CString::new("marker").unwrap();
        let v = unsafe { mquickjs_ffi::JS_GetPropertyStr(h.ctx, held.as_raw(), name.as_ptr()) };
        // JS_TAG_INT = 0 ⇒ (v & 1) == 0，值为 v >> 1
        if (v & 1) == 0 {
            ((v as i64) >> 1) as i32
        } else {
            -3
        }
    }
}