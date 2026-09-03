---
name: mquickjs-base-vs-ridl-outputs
description: ridl-builder prepare 生成 base 和 ridl 两套 QuickJS 输出，通过 feature 选择
type: architecture
created: 2026-09-03
sources: [AGENTS.md, README.md]
---

ridl-builder prepare 构建两个 QuickJS 输出变体：base（无 RIDL 扩展）和 ridl（含 RIDL 扩展）。

**为什么：** base 用于核心 crate / 测试，这些代码不能依赖 js_* 符号（RIDL 生成的绑定函数），必须与 RIDL 解耦；ridl 用于 app binary 和 JS 集成测试，需要完整的 RIDL 功能。mquickjs-sys 通过 feature `ridl-extensions` 选择链接哪个变体。这种拆分确保了核心功能的独立性，避免循环依赖。

**何时使用：** 添加新的 crate 时，判断是否需要 RIDL 功能：核心引擎层用 base（不启用 ridl-extensions feature），应用层用 ridl（启用 ridl-extensions feature）。编译失败时检查是否错误链接了不匹配的变体。
