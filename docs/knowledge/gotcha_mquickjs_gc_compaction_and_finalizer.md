---
name: mquickjs-gc-finalizer-and-compaction
description: mquickjs GC 每次 JS_GC 都压缩堆，且 sweep 确实调用 finalizer；Rust 侧 Root<T>/Traced<T> 不参与重定位是致命缺陷
type: gotcha
created: 2026-09-04
updated: 2026-09-04
sources: [deps/mquickjs/mquickjs.c:12396, deps/mquickjs/mquickjs.c:12416, deps/mquickjs/mquickjs.c:12566, deps/mquickjs/mquickjs.c:12572, deps/mquickjs/mquickjs.c:12687]
---

# mquickjs GC：压缩 + 重定位清单（含对旧结论的更正）

> **本文档更正了先前的错误结论。** 旧条目 `gotcha_mquickjs_gc_sweep_no_finalizer.md`
> 声称"GC sweep 不调用 finalizer"，经逐行核对引擎源码，**该结论错误**。

## 一、更正：sweep **确实**调用 class finalizer

`JS_GC2`（含 `gc_mark_all` + sweep + `gc_compact_heap`）的 sweep 循环中：

```c
/* reset the gc marks and mark the free blocks as free */
ptr = ctx->heap_base;
while (ptr < ctx->heap_free) {
    size = get_mblock_size(ptr);
    b = (JSFreeBlock *)ptr;
    if (b->gc_mark) {
        b->gc_mark = 0;
    } else {
        JSObject *p = (void *)ptr;
        /* call the user finalizer if needed */
        if (p->mtag == JS_MTAG_OBJECT && p->class_id >= JS_CLASS_USER &&
            ctx->c_finalizer_table[p->class_id - JS_CLASS_USER] != NULL) {
            ctx->c_finalizer_table[p->class_id - JS_CLASS_USER](ctx, p->u.user.opaque);
        }
        /* merge all the consecutive free blocks */
        ...
    }
    ptr += size;
}
```

来源：`mquickjs.c:12405-12435`（finalizer 调用在 `12416-12419`）。

**结论**：未标记（不可达）且 `class_id >= JS_CLASS_USER` 的对象，其 class finalizer
**会在本次 GC 的 sweep 阶段被调用**，不必等到 `JS_FreeContext`。

**推论**：
- native opaque（`Box<dyn Trait>`）**可以**在 GC 时释放，前提是 class 有 finalizer 且
  finalizer 表项非空。
- "必须靠显式 `dispose()` 或短生命周期 context 规避泄漏"的缓解方案是**基于错误前提**
  设计的，需要重新评估。
- 旧的「分配压力测试 32000 次迭代 SIGSEGV 是 opaque 泄漏所致」的归因需要重新诊断——
  更可能的原因是下面第二条（悬垂指针），而不是 finalizer 缺失。

## 二、新发现（比旧结论严重得多）：`JS_GC` 每次都压缩堆，而 Rust 侧的
##    `Root<T>` / `Traced<T>` **不参与重定位**

`JS_GC` → `JS_GC2(ctx, TRUE)` → `gc_mark_all` + **`gc_compact_heap`**（`mquickjs.c:12687`）。
即**每一次 `JS_GC` 都会移动堆上的对象块**（memmove 到新位置）。

`gc_compact_heap` 的重定位（threading）清单（`mquickjs.c:12572-12608`）**只有**：

| 被线程化的位置 | 说明 |
|---|---|
| `ctx->unique_strings` … `class_proto + 2*class_count` | context 内建字段 |
| `ctx->string_pos_cache[i].str` | 字符串位置缓存 |
| `ctx->sp` … `ctx->stack_top` | JS 值栈 |
| **`ctx->top_gc_ref` / `ctx->last_gc_ref` 链上的 `ref->val`** | **`JSGCRef` 链 —— 引擎官方的跨 GC 持有机制** |
| `ctx->parse_state` 各字段 | 解析器状态 |

**不在此清单中**：`ctx->gc_mark`（通过 `JS_SetContextGCMark` 注册的回调）所标记的值。

`ctx->gc_mark` 只在 **mark 阶段**被调用（`mquickjs.c:12396`：
`ctx->gc_mark(ctx, ctx->gc_mark_opaque, &mf)`），其标记的值会被**保活**，但
**不会被重定位**。

### 后果（严重）

本项目的：

- `RootsRegistry` / `Root<T>`（经 `JS_SetContextGCMark` 注册）
- `Traced<T>`（用户 opaque 字段，经 class `gc_mark` 回调标记）
- `AsyncTaskManager::callback_registry`（直接存裸 `JSValue`）

全都把裸 `JSValue` 存在**引擎不知道的 Rust 内存**里。它们：

- ✅ 在 mark 阶段被标记 → 对象**不会被回收**
- ❌ 在 compact 阶段**不被线程化** → 对象**被移动后，Rust 侧持有的仍是旧地址**

**即：任何一次 `JS_GC` 之后，所有 `Root<T>` / `Traced<T>` / callback registry 中的
`JSValue` 都可能变成悬垂指针。** 这是静默的内存损坏，不是"泄漏"。

现有 GC 测试之所以通过，是因为它们**没有在持有 Root 的同时触发会移动目标对象的
压缩**，或在移动后没有真正解引用旧地址。

### 正确做法

引擎已经提供了**压缩安全**的机制：`JSGCRef` + `JS_AddGCRef`（`mquickjs.c:459`，
在 `12593/12596` 的线程化清单中）。

- 需要跨 GC 持有 JS 值时，应使用 `JSGCRef`，**而不是**自建 registry。
- `Traced<T>` 若必须存在于用户 opaque 中，需要额外的间接层（例如 opaque 持有
  `JSGCRef`，或改用引擎认可的 holder），不能直接存裸 `JSValue`。

> 本项目早先的决策笔记曾判断"mquickjs 没有 `JS_DupValue`/`JS_FreeValue`，所以必须
> 自建 Root registry"。该判断只看到了"没有 refcount"这一半，**漏掉了 `JSGCRef` 这一
> 压缩安全的官方机制**，导致自建了一套在压缩式 GC 下不安全的方案。

## 三、复现与验证方法（待补）

- [ ] 写一个测试：建立 `Root<T>` 持有一个对象 → 制造垃圾触发 `JS_GC`（使目标对象被
      memmove 到新地址）→ 通过 Root 解引用该对象 → 观察是否读到错误数据/崩溃。
- [ ] 用 `DUMP_GC` 编译引擎，观察 `AFTER: heap size=...` 与地址变化。
- [ ] 修正后重跑 `tests/gc_root_cycle.rs` 与 `tests/gc_traced.rs`。

## 四、与 quickjs 的区别（更正版）

| 维度 | quickjs | mquickjs |
|---|---|---|
| 回收机制 | refcount + cycle GC | 纯 tracing GC，无 refcount |
| 堆压缩 | 不移动对象 | **每次 `JS_GC` 都压缩（移动对象）** |
| finalizer | refcount→0 时立即调用 | 不可达对象在 **sweep 阶段**调用 |
| 跨 GC 持有 | `JS_DupValue` | **`JSGCRef` + `JS_AddGCRef`**（必须用这个） |