---
name: quickjs-rom-ridl-mechanism
description: QuickJS ROM 机制与 RIDL 扩展的关系，当初实现时未充分理解 ROM
type: gotcha
created: 2026-09-03
sources: [AGENTS.md]
---

当前 RIDL 会被编译进 ROMClass 的 props 与 proto_props，但当初实现时并未充分理解 ROM 机制。

**为什么：** ROM（Read-Only Memory）机制是 QuickJS 的编译期优化，将常量对象/类定义预编译进 ROM，避免运行时构造。RIDL 扩展与标准库的关系、编译阶段考量需要重新审视，确保 RIDL 生成的类定义在 ROM/标准库上下文中正确工作。

**何时使用：** 修改 RIDL 生成逻辑、添加新的 RIDL 扩展、或调试 RIDL 类在运行时的行为异常时，需要考虑 ROM 机制的影响。参考 QuickJS 官方文档关于 ROM 的说明。
