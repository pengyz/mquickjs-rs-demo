---
name: mquickjs-gc-mark-signature
description: mquickjs 的 gc_mark C 签名是 (ctx, void *opaque, const JSMarkFunc *mf)，不是 QuickJS 的 (ctx, JSValue, opaque, mf)
type: gotcha
created: 2026-09-03
sources: [deps/mquickjs/mquickjs.h, deps/mquickjs/example_stdlib.c, 8a76113 复审]
---

mquickjs **不是** quickjs：两者的 gc_mark 回调签名不同。

- **mquickjs 实际签名**：`(JSContext *ctx, void *opaque, const JSMarkFunc *mf)`
  - 定义见 `deps/mquickjs/mquickjs.h` 的 `JSCMark`/`JSContextGCMark` typedef
  - 用法范例见 `deps/mquickjs/example_stdlib.c` 的 `js_rectangle_gc_mark`
- **易错签名**（QuickJS 的 per-class finalizer 风格）：`(JSContext *ctx, JSValue obj, void *opaque, const JSMarkFunc *mf)`

教训：2026-09-03 的设计文档与实现计划草稿（Task 0.7）写的正是错误的 4 参签名，
实现者以引擎实际 API 为准才避免固化错误。生成 `JS_CLASS_DEF` 时 gc_mark 是
**第 9 个位置参数**（紧跟 finalizer 之后），无 Traced 字段时置 `NULL`。

规则：凡写 mquickjs C 回调签名，先 grep `deps/mquickjs/mquickjs.h` 的 typedef，
不要凭 quickjs 上游知识或文档草稿假设。
