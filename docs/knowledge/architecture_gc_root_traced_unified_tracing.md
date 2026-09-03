---
name: gc-root-traced-unified-tracing
description: Root<T> + Traced<T> 统一 tracing GC 设计，用户不感知 mark
type: architecture
created: 2026-09-03
sources: [2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md]
---

在 mquickjs-rs 引入 Root<T> 和 Traced<T>，实现统一 tracing GC，用户不需要手动实现 mark。

**为什么：** 引擎层已有 user class gc_mark 和 context-level JS_SetContextGCMark 能力，但现有 Local<T> 不是 root，Global<T> 通过 JSGCRef 链入引擎 roots。新需求是用户在 Rust 结构中保存 JSValue 时，对象内字段可以不使用 Global，异步任务/全局队列等堆外结构也可稳定保活且可撤销。Root<T> 作为跨 await/跨对象持有 JSValue 的主路径，Drop 自动撤销根；Traced<T> 用于 user class 的 opaque 内保存 JSValue 字段，由生成代码自动实现 gc_mark。

**何时使用：** 需要跨 async/跨调用持有 JSValue 时用 Root<T>；在 RIDL 生成的 user class 内部存储 JSValue 字段时用 Traced<T>。两者配合通过"刻意制造 A↔Rust↔B 强引用环"验证：释放 Root 后环可回收，Root 持有时环保活。
