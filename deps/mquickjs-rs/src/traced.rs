use crate::handles::local::Local;
use crate::handles::scope::Scope;
use crate::mquickjs_ffi;
use crate::roots::Root;

/// RIDL 用户 class opaque 中的 GC 追踪字段。
///
/// # 语义
///
/// `Traced<T>` 保存一个 JS 值，其生命周期与其所属的 opaque 绑定：
/// opaque 存活时值被保活；opaque 被回收（class finalizer 运行 → Box drop）时，
/// `Traced` 一并释放，值即可被回收。
///
/// # 实现：委托给 `Root<T>`
///
/// 本类型是 [`Root`] 的薄封装。两者提供同一能力（跨宿主生命周期的 GC 安全句柄），
/// 因此共用同一份**已验证**的实现，避免重复实现带来的偏差。
///
/// ## 为什么不能自己存裸 `JSValue`
///
/// 早期实现把裸 `JSValue` 直接存在 opaque 里，只靠 owner 的 `gc_mark` 回调保活。
/// 该做法**不安全**：mquickjs 的 `JS_GC` 每次都会压缩堆
/// （`JS_GC2` → `gc_compact_heap`，用 memmove 移动存活对象），
/// 而 `gc_mark` 回调**只在 mark 阶段被调用**（`mquickjs.c:12319`），
/// 无法注册重定位。于是压缩后 opaque 里的 `JSValue` 仍指向旧地址 —— 悬垂。
///
/// 复现见 `deps/mquickjs-rs/tests/gc_compaction.rs` 的
/// `gc_compaction_does_not_relocate_traced`：修复前同一对象在引擎权威值
/// （`JSGCRef`）与本类型中相差约 70 KB。
///
/// `Root` 基于引擎的 `JSGCRef`，该链在 mark（`mquickjs.c:12319-12323`）
/// 与重定位（`mquickjs.c:12592-12596`）**两个阶段都被扫描**，
/// 是 mquickjs 下唯一压缩安全的跨 GC 持有机制。
///
/// ## 为什么 opaque 内可以安全持有它
///
/// RIDL 生成的 opaque 是 `Box::into_raw(holder)` 得到的**稳定堆地址**
/// （见 `rust_glue.rs.j2` 的 `JS_SetOpaque(ctx, obj, holder_ptr)` 调用），
/// 不随 JS 对象的压缩移动。因此其内部 `Root` 所持有的 `Box<JSGCRef>`
/// 地址稳定，引擎的 `last_gc_ref` 链表不会因压缩而损坏。
///
/// # Safety Model
///
/// **不变式**：`Traced<T>` 生命周期 ≤ 所属 JS 对象生命周期。
///
/// 由以下保证：
/// 1. JS 对象可达 → opaque 有效 → `Traced<T>` 有效
/// 2. JS 对象不可达 → class finalizer 运行 → opaque Box drop → `Traced<T>` drop
///    → `JSGCRef` 从引擎链表摘除
///
/// 每个 RIDL class 都会**无条件**注册 `_finalizer`
/// （`mquickjs_ridl_register.h.j2` 的 class_def 表），因此 (2) 必然发生。
///
/// **必须仅在 RIDL 用户 class opaque 内使用。**
///
/// ## 违反不变式的场景（必须避免）
///
/// ```rust,no_run
/// //  把 Traced 取出 opaque
/// // let traced = opaque.held.take(); // opaque 之后可能被 finalize
/// // traced 届时已悬垂
/// ```
pub struct Traced<T = crate::handles::local::Value>(Root<T>);

impl<T> Traced<T> {
    /// 从 `Local<T>` 创建 `Traced<T>`。
    ///
    /// 需要显式传入 `Scope`：注册 GC 安全句柄需要 JS context。
    /// 调用方所在的方法因此需要 `needs_scope`（即接收 `any` / `object` 参数）。
    ///
    /// # Safety
    ///
    /// 调用方必须确保本 `Traced` 存在 RIDL 用户 class 的 opaque 中，
    /// 从而在其 owner 被回收时被 drop（否则 `JSGCRef` 会泄漏，
    /// 导致被引用的对象永不回收）。
    pub fn new(scope: &Scope<'_>, v: Local<'_, T>) -> Self {
        Self(Root::new(scope, v))
    }

    /// 返回该字段当前持有的 `JSValue`。
    ///
    /// **这是 GC 压缩重定位后的最新地址**，而非创建时的副本。
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.0.as_raw()
    }

    /// 该字段是否持有一个值。
    pub fn is_some(&self) -> bool {
        // JSValue 是整数编码；0 不是合法值。
        self.0.as_raw() != 0
    }

    /// 兼容旧接口的空实现。
    ///
    /// # 为什么是空实现
    ///
    /// 本类型现在基于 `JSGCRef`，其链**由引擎在 mark 阶段自动扫描**
    /// （`mquickjs.c:12319-12323`），因此无需再由 RIDL 生成的
    /// `gc_mark` 回调手动标记。
    ///
    /// 保留本方法是为了不改动 RIDL 模板对 `Traced` 字段生成的调用
    /// （`rust_api.rs.j2` 的 `Opaque::gc_mark`）。
    /// 对同一个值重复标记在 mark-sweep GC 中是幂等的，无副作用。
    ///
    /// # Safety
    ///
    /// `mf` 应为引擎提供的 mark 函数指针（本实现不再解引用它）。
    pub unsafe fn gc_mark(&self, _mf: *const mquickjs_ffi::JSMarkFunc) {
        // 引擎已通过 JSGCRef 链标记并重定位该值，无需手动标记。
        //
        // 注意：这里**不能**退化为"手动 mark 一下 self.as_raw()"，
        // 因为那只解决保活、不解决重定位 —— 正是被本修复取代的错误做法。
    }
}