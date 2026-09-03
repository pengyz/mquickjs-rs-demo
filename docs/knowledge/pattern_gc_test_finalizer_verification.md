---
name: gc-test-finalizer-verification
description: GC 回收测试通过 finalizer 验证对象被回收
type: pattern
created: 2026-09-03
sources: [2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md, tests/gc_root_cycle.rs]
---

GC 回收测试必须通过 finalizer 验证对象是否被回收，而非仅依赖引用计数或内存占用。

**为什么：** 在 tracing GC 中，对象是否被回收取决于可达性，而非引用计数。测试"释放 Root 后环可回收"需要明确证据：设置 JS_SetClassFinalizer，在 finalizer 中增加全局计数器或设置标志位，触发 GC 后检查 finalizer 是否被调用。仅检查对象引用为空不足以证明回收发生（可能只是解除了 Rust 侧引用，但对象仍在 GC 堆中）。

**何时使用：** 编写 GC 相关测试时，尤其是验证循环引用、Root/Global 管理、Traced 字段标记等场景。测试模式：构造引用环 → 释放外部引用 → 触发 GC → 断言 finalizerCount 增长。
