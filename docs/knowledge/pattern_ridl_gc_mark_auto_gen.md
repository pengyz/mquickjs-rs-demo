---
name: ridl-gc-mark-auto-gen
description: RIDL 自动生成 user class gc_mark 枚举 Traced<T> 字段
type: pattern
created: 2026-09-03
sources: [2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md]
---

RIDL 生成侧支持 Traced<T> 字段语义，自动生成 user class 的 gc_mark 实现。

**为什么：** Traced<T> 用于对象内字段，不 root（不进入 RootsRegistry），也不要求 JSGCRef。它依赖 owning JS 对象的 gc_mark：当对象 A 可达时，gc_mark 被调用并标记 Traced 字段中的 JSValue（如 B）；当 A 不可达时，gc_mark 不会被调用，字段不会阻止回收。生成代码遍历 opaque struct 中所有 Traced<T> 字段，在 gc_mark 回调中调用 mark_value。

**何时使用：** 在 RIDL 定义 user class 时，opaque 内需要保存 JSValue 字段的场景。用户不手写 mark，生成代码自动处理。测试时通过 finalizer 验证"释放外部引用后对象可回收"。
