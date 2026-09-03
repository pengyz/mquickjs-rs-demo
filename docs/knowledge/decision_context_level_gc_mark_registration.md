---
name: context-level-gc-mark-registration
description: Context 创建时一次性注册 gc_mark，遍历 RootsRegistry 中所有 roots
type: decision
created: 2026-09-03
sources: [2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md, context.rs]
---

Root<T> 的实现采用 context-level gc_mark 路径，而非每个 Root 单独注册。

**为什么：** Context 创建时安装 JS_SetContextGCMark(ctx, opaque, mark_fn)，opaque 指向 RootsRegistry（挂在 ContextInner），mark_fn 遍历 registry 中当前存在的 roots，逐个 mark_value。这比每个 Root 实例调用 JS_AddGCRef/JS_DeleteGCRef 更高效（一次注册 vs N 次注册），且 Drop 只需从 registry 移除 slot，不需要调用引擎 API。

**何时使用：** 实现类似 Root<T> 的 context-bound GC root 机制时，优先考虑 context-level 注册 + registry 模式，而非 per-instance 注册。
