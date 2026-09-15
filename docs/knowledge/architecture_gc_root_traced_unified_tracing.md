---
name: gc-root-traced-unified-tracing
description: Root<T> 与 Traced<T> 统一基于引擎 JSGCRef；用户不感知 mark，也不再有 class gc_mark
type: architecture
created: 2026-09-03
updated: 2026-09-04
sources: [deps/mquickjs-rs/src/roots.rs, deps/mquickjs-rs/src/traced.rs, deps/mquickjs-rs/tests/gc_compaction.rs]
---

# Root<T> / Traced<T>：统一基于 JSGCRef 的跨 GC 持有

用户不需要手动实现 mark，也不需要知道引擎的重定位细节。

## 核心约束（决定了唯一可行设计）

mquickjs 的 `JS_GC` → `JS_GC2` → `gc_mark_all` + **`gc_compact_heap`**：
**每一次 GC 都会压缩堆**（memmove 移动存活对象）。

引擎只重定位"它自己知道的"指针，清单（`mquickjs.c:12572-12608`）只含：
context 内建字段、`string_pos_cache`、JS 值栈、**`JSGCRef` 链**、`parse_state`。

`JS_SetContextGCMark` 注册的回调**只在 mark 阶段被调用**
（`mquickjs.c:12319`），**无法注册重定位**。

⇒ 任何"只靠 gc_mark 保活"的方案在压缩后都会悬垂。
⇒ **`JSGCRef` 是 mquickjs 下唯一压缩安全的跨 GC 持有机制**
（mark: `12319-12323`，重定位: `12592-12596` 两阶段都扫描）。

## 现行设计

- **`Root<T>`**：每个 root 一个 `Box<JSGCRef>`，经 `JS_AddGCRef` 链入
  `ctx->last_gc_ref`。`as_raw()` 读 `ref->val` —— 即**重定位后的最新值**。
  Drop 时 `JS_DeleteGCRef`，受 `ContextInner.alive` 护栏保护
  （`Context::drop` 在 `JS_FreeContext` 前把 alive 置 false）。
- **`Traced<T>`**：`Root<T>` 的**薄封装**。两者提供同一能力（跨宿主生命周期的
  GC 安全句柄），共用同一份已验证实现。用于 RIDL user class 的 opaque 内字段；
  其生命周期与 opaque 绑定（opaque 由 class finalizer drop 时一并释放）。

**opaque 内可以安全持有它**：RIDL 生成的 opaque 是
`Box::into_raw(holder)` 得到的**稳定堆地址**（内联在 JS 对象里的只有一个 8 字节
`void*`），不随压缩移动，因此其中的 `Box<JSGCRef>` 地址稳定，
引擎的 `last_gc_ref` 链表不会损坏。

## 语义与旧设计的差异

旧 `Traced` 语义是"owner 可达 ⇒ 字段被标记保活"。
新实现下 `Traced` 是一个**显式 root**：owner 被回收时其 finalizer 会 drop 它，
从而摘除 root。生命周期边界一致，但被引用对象会**晚一轮 GC** 才可回收。

## 何时使用

- 跨 async / 跨调用持有 `JSValue` → `Root<T>`
- 在 RIDL user class opaque 内保存 JSValue 字段 → `Traced<T>`

## 验证

- `deps/mquickjs-rs/tests/gc_compaction.rs`：差分对比「`JSGCRef` 权威值 vs
  `Root` / `Traced`」。修复前相差约 70 KB，修复后完全一致。
- `deps/mquickjs-rs/tests/gc_root_cycle.rs`：压缩后**真实解引用**并断言值正确。
- `tests/gc_traced.rs`：RIDL 端到端，节点**可达**时 GC + 压缩后读 `Traced` 字段。

> 历史：早期实现让 `Traced` 存裸 `JSValue` + 依赖 class `gc_mark`，
> 该机制既有压缩悬垂缺陷，其生成代码还有 4 参 vs 3 参的致命 ABI 缺陷。
> 详见 [`gotcha_mquickjs_gc_compaction_and_finalizer.md`](gotcha_mquickjs_gc_compaction_and_finalizer.md)
> 与 [`pattern_ridl_gc_mark_auto_gen.md`](pattern_ridl_gc_mark_auto_gen.md)。