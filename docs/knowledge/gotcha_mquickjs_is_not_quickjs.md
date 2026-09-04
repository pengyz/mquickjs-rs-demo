---
name: mquickjs-is-not-quickjs
description: mquickjs 是独立项目，不是 QuickJS 的 fork/子集——共享代码渊源但架构完全不同
type: gotcha
created: 2026-09-04
sources: [deps/mquickjs/README.md, .gitmodules]
---

## mquickjs ≠ QuickJS

**关系**：共享部分代码基础（同源，Bellard/Gordon 版权），但是独立项目。

**来源**：`https://github.com/pengyz/mquickjs`（git submodule），不是 QuickJS 的 fork。

**关键架构差异**：

| 维度 | QuickJS | mquickjs |
|---|---|---|
| 目标 | 通用嵌入式 JS 引擎 | 极致嵌入式（10kB RAM） |
| GC | 引用计数 + 循环检测 | 纯 tracing GC |
| JS 子集 | 完整 ES2023 | ES5 子集 |
| 公开 API | JS_FreeValue/JS_DupValue | 无（tracing GC 管理） |
| 类注册 | 运行时 JS_NewClass | 编译期 stdlib-def 表 |
| 异步 | Promise/async/await | 无 |
| finalizer 时机 | refcount→0 时 | context teardown 时 |

**规则**：
- 不要假设 mquickjs 有 QuickJS 的特性（async、WeakRef 等）
- 不要引用 QuickJS 文档/行为来推断 mquickjs
- 凡涉及 mquickjs API，先 grep `deps/mquickjs/mquickjs.h` 验证
