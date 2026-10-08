---
name: quickjs-rom-ridl-mechanism
description: QuickJS ROM 机制与 RIDL 扩展的关系，当初实现时未充分理解 ROM
type: gotcha
created: 2026-09-03
sources: [AGENTS.md, docs/planning/2026/2026-10-08-todo-items-resolution.md]
---

当前 RIDL 会被编译进 ROMClass 的 props 与 proto_props，但当初实现时并未充分理解 ROM 机制。

**为什么：** ROM（Read-Only Memory）机制是 QuickJS 的编译期优化，将常量对象/类定义预编译进 ROM，避免运行时构造。RIDL 扩展与标准库的关系、编译阶段考量需要重新审视，确保 RIDL 生成的类定义在 ROM/标准库上下文中正确工作。

**何时使用：** 修改 RIDL 生成逻辑、添加新的 RIDL 扩展、或调试 RIDL 类在运行时的行为异常时，需要考虑 ROM 机制的影响。参考 QuickJS 官方文档关于 ROM 的说明。

## singleton 是匿名 class：无 class id、无可寻址 proto（Phase C 复核实证）

- singleton 经 `JS_OBJECT_DEF(name, props)` 注册（register.h.j2），是
  **class_id=NULL 的匿名 class**：没有 class id，也没有可寻址的 prototype。
- 因此 singleton 上**永远不支持 `proto var` / proto property**——validator
  明确拒绝（validator/mod.rs `validate_singleton_js_fields`），错误信息说明
  原因。给 singleton "建 class" 以获得 proto 属于 4-6 天的独立特性
  （class id 分配、ROM JS_CLASS_DEF、require 表 ensure_class_ids、romclass
  map 全链改造），不要顺手做。
- singleton 的 plain `var` 字段（js-only）安装落点是
  `mquickjs_ridl_register.c` 的 `JS_RIDL_StdlibInit`：
  `JS_GetGlobalObject` → `JS_GetPropertyStr(singleton 名)` → 逐字段
  `JS_SetPropertyStr`（writable）。**仅 GLOBAL 模式**（`module_decl.is_none()`）
  生成安装代码——module 模式 singleton 今天未注册到任何 JS 可达位置。
  安装发生在用户脚本之前（Context::new 内），字段先于任何方法调用可读。

## C 侧字面量 emission 的三个坑（Phase C 实测撞上）

1. **`JS_NewBool` 是 1 参 static inline**（mquickjs.h，无 ctx 参数），
   与 `JS_NewInt32(ctx, v)`/`JS_NewString(ctx, s)` 不一致。生成的 C 里写
   `JS_NewBool(ctx, ...)` 直接编译失败。
2. **生成的 C 文件没有 stdbool**：RIDL bool 字面量（`true`/`false`）必须
   映射为 `1`/`0`，不能裸写 `true`（未声明标识符）。
3. **`init_literal` 是解码后原文**（parser decode 后含真实的 `"`、换行、
   反斜杠），裸嵌模板会生成非法 C 源码。必须过 `escape_c_string` filter
   再嵌入（filters.rs；Rust 侧对应 `escape_rust_string`）。

## js 字段字面量类型支持矩阵（有意不对称）

| 位置 | validator 允许 | 实际结局 |
|------|---------------|----------|
| class js_fields | Bool/I32/I64/F32/F64/String/Null/Any/Optional/Custom(=null) | I64/F32/F64 在 glue 模板 `unreachable!()` panic（已知地雷，另立缺陷单） |
| singleton var | **仅 {i32, bool, string, null}** | 其余类型编译期明确拒绝，错误信息列出支持集 |

singleton 侧更严是有意的：在地雷上方再加一层"允许但 panic"没有意义。
另注意 parser 字符串转义约定：`\"`→引号（单反斜杠）；其余转义用
**双反斜杠形式**（`\\n`→LF、`\\\\`→两个反斜杠），见
`decode_ridl_string_literal`（parser/mod.rs）。grammar 的 `string_literal`
已支持 `\"`（严格超集，无转义的旧字面量解析不变）。
