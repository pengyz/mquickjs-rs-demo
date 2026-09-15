---
name: context-level-gc-mark-registration
description: 【已作废】曾用 JS_SetContextGCMark 一次注册遍历所有 roots —— 该路径无法重定位，已改为 per-instance JSGCRef
type: decision
created: 2026-09-03
updated: 2026-09-04
status: retracted
sources: [docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md, deps/mquickjs-rs/src/roots.rs]
---

# ️ 本条决策已作废（RETRACTED）

**原决策**：Root 采用 context-level 注册 —— 用 `JS_SetContextGCMark` 安装一个
回调，遍历 `RootsRegistry` 里所有 root 逐个 `mark_value`；理由是"一次注册
比 N 次 `JS_AddGCRef`/`JS_DeleteGCRef` 更高效"。

**该设计不安全，且'更高效'的论证是错的。**

## 为什么错

`JS_SetContextGCMark` 注册的回调**只在 mark 阶段被调用**
（`mquickjs.c:12319`），而且 `mark_value` 的语义**只有标记、没有重定位**。
引擎在 `gc_compact_heap` 中移动存活对象时，只会重定位它自己知道的指针
（清单：context 内建字段、`string_pos_cache`、JS 值栈、**`JSGCRef` 链**、
`parse_state`）—— 自建 registry 不在其中。

⇒ registry 里的 `JSValue` 在**每次** `JS_GC` 之后都指向旧地址。
这不是"性能取舍"，而是**静默内存损坏**。

复现（差分对比，修复前）：
```
root=0x72fec40159a1   relocated=0x72fec4004571   same=false   ← 相差 70640 字节
```

## 现行决策

**per-instance `JSGCRef`**（`deps/mquickjs-rs/src/roots.rs`）：

- 每个 root 一个 `Box<JSGCRef>`，经 `JS_AddGCRef` 链入 `ctx->last_gc_ref`
- `as_raw()` 读 `ref->val` —— 引擎会在压缩时直接更新它
- Drop 时 `JS_DeleteGCRef`

代价是每个 root 一次链表插入/摘除（O(n) 最坏），但这是**唯一正确**的路径：
`JSGCRef` 链在 mark 与重定位两个阶段都被引擎扫描。

**教训**：在 tracing GC 上做"更高效的等价方案"之前，先确认**引擎承诺了哪些
不变量**。"标记"与"重定位"是两件事，前者可由用户回调完成，后者不能。

## 何时使用

实现 context-bound GC root 时，**必须**使用引擎提供的跨 GC 持有机制
（这里是 `JSGCRef`）。不要自建 registry + mark 回调 —— 那只解决保活、不解决
对象移动。

参见 [`gotcha_mquickjs_gc_compaction_and_finalizer.md`](gotcha_mquickjs_gc_compaction_and_finalizer.md)。