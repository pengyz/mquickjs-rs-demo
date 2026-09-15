---
name: roots-registry-vec-option
description: 【已作废】RootsRegistry 曾用 Mutex<Vec<Option<JSValue>>>+gc_mark —— 该设计在压缩式 GC 下不安全，已改为 JSGCRef
type: decision
created: 2026-09-03
updated: 2026-09-04
status: retracted
sources: [docs/knowledge/architecture_base_vs_ridl_variant_selection.md, deps/mquickjs-rs/src/roots.rs]
---

# ️ 本条决策已作废（RETRACTED）

**原决策**：`RootsRegistry` 采用 `Mutex<Vec<Option<JSValue>>>`，配合
`JS_SetContextGCMark` 注册的 `gc_mark` 回调遍历标记，Drop 时置空 slot。

**该设计不安全。** `gc_mark` 回调**只在 mark 阶段被调用**，无法注册重定位；
而 mquickjs 的 `JS_GC` **每次都会压缩堆**（`gc_compact_heap` 用 memmove 移动存活对象）。
因此 registry 里的 `JSValue` 在压缩后指向旧地址 —— **静默内存损坏**。

复现（差分对比，修复前）：
```
root=0x72fec40159a1   relocated=0x72fec4004571   same=false   ← 相差 70640 字节
```
见 `deps/mquickjs-rs/tests/gc_compaction.rs`。

## 现行设计

`RootsRegistry` 现基于引擎的 **`JSGCRef`**（每个 root 一个 `Box<JSGCRef>`），
该链在 mark（`mquickjs.c:12319-12323`）与重定位（`mquickjs.c:12592-12596`）
**两个阶段都被扫描**，是 mquickjs 下唯一压缩安全的跨 GC 持有机制。

- `insert`：`Box<JSGCRef>` + `JS_AddGCRef`（Box 保证地址稳定，引擎持有 `&mut *r`）
- `as_raw`：读 `ref->val`（即重定位后的最新值）
- `remove`：`JS_DeleteGCRef`，并受 `ContextInner.alive` 护栏保护
- `JS_SetContextGCMark` 注册已移除（被 JSGCRef 取代）

**请阅读**：[`architecture_base_vs_ridl_variant_selection.md`](architecture_base_vs_ridl_variant_selection.md)、
[`gotcha_mquickjs_gc_compaction_and_finalizer.md`](gotcha_mquickjs_gc_compaction_and_finalizer.md)

> 注：`Vec<Option<T>>` 作为"支持删除的索引池"本身仍是合理选择；
> 本条作废针对的是**用它存放裸 `JSValue` 并靠 gc_mark 保活**这一用法。