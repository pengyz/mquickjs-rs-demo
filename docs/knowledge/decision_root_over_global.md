---
name: root-over-global
description: Root<T> 优于 Global<T> 作为跨 await 持有 JSValue 的推荐方案
type: decision
created: 2026-09-03
sources: [2026-01-25-v1-c3-mquickjs-rs-root-and-traced.md]
---

推荐使用 Root<T> 而非 Global<T> 作为跨 await/跨调用持有 JSValue 的主路径。

**为什么：** Root<T> 采用 context-level gc_mark 路径，一次注册遍历所有 roots，Drop 自动撤销；Global<T> 通过 JSGCRef 每个实例单独注册（JS_AddGCRef/JS_DeleteGCRef），管理成本更高。Root<T> 不跨 Context（通过 ctx_id 校验）、不跨线程（!Send/!Sync），语义更清晰。Global<T> 保持兼容作为底层/特殊场景使用（如需与现有 JSGCRef 生态对齐）。

**何时使用：** 异步任务、全局队列等需要跨 await 持有 JSValue 时，优先使用 Root<T>。仅在明确需要与 JSGCRef 生态对齐或特殊场景时才使用 Global<T>。
