---
name: mquickjs-gc-sweep-no-finalizer
description: 【已作废/结论错误】原称 sweep 不调用 finalizer —— 已被 gotcha_mquickjs_gc_compaction_and_finalizer 更正
type: gotcha
created: 2026-09-04
updated: 2026-09-04
status: retracted
sources: [docs/knowledge/gotcha_mquickjs_gc_compaction_and_finalizer.md]
---

# ️ 本条结论已作废（RETRACTED）

**原结论**：mquickjs 的 GC sweep 释放 JS 对象但不调用 finalizer，因此 native opaque
会泄漏到 context teardown。

**该结论错误。** 经逐行核对引擎源码，sweep 循环中**确实调用** class finalizer：

```c
/* call the user finalizer if needed */          /* mquickjs.c:12416 */
if (p->mtag == JS_MTAG_OBJECT && p->class_id >= JS_CLASS_USER &&
    ctx->c_finalizer_table[p->class_id - JS_CLASS_USER] != NULL) {
    ctx->c_finalizer_table[p->class_id - JS_CLASS_USER](ctx, p->u.user.opaque);
}
```

**请阅读更正后的条目**：
[`gotcha_mquickjs_gc_compaction_and_finalizer.md`](gotcha_mquickjs_gc_compaction_and_finalizer.md)

该条目同时记录了一个**远比原结论严重**的真实缺陷：
`JS_GC` 每次都压缩堆，而 `Root<T>` / `Traced<T>` / callback registry 存的裸 `JSValue`
**不参与重定位**，压缩后即悬垂 —— 这是内存损坏，不是泄漏。

**保留本文件的原因**：避免其他文档/记忆继续引用这条错误结论。