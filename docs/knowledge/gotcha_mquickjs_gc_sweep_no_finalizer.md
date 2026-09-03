---
name: mquickjs-gc-sweep-no-finalizer
description: mquickjs GC sweep 释放 JS 对象但不调用 finalizer，opaque Box 泄漏到 context teardown
type: gotcha
created: 2026-09-04
sources: [tests/gc_traced.rs, deps/mquickjs/mquickjs.c:12396]
---

## mquickjs GC sweep 不调用 finalizer

**现象**：
- `JS_GC`（sweep）回收不可达 JS 对象内存，但不调用 class finalizer
- `JS_FreeContext`（teardown）才调用 finalizer
- 分配压力测试：32000+ 次迭代导致 SIGSEGV（opaque Box 泄漏 → 原生堆溢出）

**根因**：
- mquickjs 的 `gc_compact_heap` 只做 mark+compact，不遍历 finalizer 表
- finalizer 调用只在 `JS_FreeContext` 的堆遍历中（`mquickjs.c:3673`）
- 这是 Bellard 的设计选择（嵌入式引擎，简化 GC）

**影响**：
- GC sweep 释放 JS 对象内存 ✓
- 但 native opaque（Box<dyn Trait>）泄漏到 context teardown
- 长期运行的 context 会累积 native 内存泄漏

**缓解**：
- 短生命周期 context：创建→执行→销毁，泄漏量可控
- 显式 dispose 模式：RIDL 方法暴露 `dispose()` 释放 native 资源
- 限制单 context 内的对象创建数量（~10000 安全）

**测试策略**：
- 分配压力测试限制迭代次数（10000）
- finalizer 验证只在 teardown 后进行
- mid-life 用行为探针（对象仍可访问）而非 finalizer 计数

**与 quickjs 的区别**：
- quickjs：refcount + 循环检测 GC，refcount→0 时立即调用 finalizer
- mquickjs：纯 tracing GC，无 refcount，finalizer 延迟到 teardown
