<!-- planning-meta
status: 未复核
tags: build, engine, ridl
replaced_by:
- docs/ridl/overview.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `engine` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# 规划：拆分 RIDL register 头文件（decl/def/umbrella）以适配 mquickjs ROM 构建

日期：2026-01-19

## 1. 背景与问题复现

当前 `ridl-builder prepare --profile framework` 在 RIDL ROM 构建阶段失败，核心报错：

- `mqjs_ridl_stdlib.h` 初始化 `js_c_function_table[]` 时引用 `js_test_require_1_0_module_constructor`，但编译 `mqjs_stdlib_impl.c` 时提示：
  - `error: ‘js_test_require_1_0_module_constructor’ undeclared here (not in a function)`

该错误可用 mquickjs-build 的真实编译参数稳定复现：

- mquickjs-build 在步骤 6 编译 `mqjs_stdlib_impl.c` 时会强制：
  - `-DMQUICKJS_RIDL_DECLARE_ONLY`
  - `-include mquickjs_ridl_register.h`

同时，步骤 2 生成 ROM 表的 host tool（`mqjs_ridl_stdlib`）并不定义 `MQUICKJS_RIDL_DECLARE_ONLY`，因此它能看到完整的 module ctor 定义并把符号名写进 ROM 输出头 `mqjs_ridl_stdlib.h`。

最终形成结构性矛盾：

- ROM 输出（`mqjs_ridl_stdlib.h`）**引用了** `js_*_module_constructor`
- 但 `DECLARE_ONLY` 模式下，被 include 的 register 头 **没有提供该符号的声明**（原型被错误 gate 掉）

## 2. 根因分析（结构性问题）

当前生成的 `mquickjs_ridl_register.h` 同时承担了三类职责：

1) **ROM build 扫描输入**：需要在 host tool 编译期可见的 `JSClassDef/JSPropDef` 静态定义、module ctor 定义等。
2) **运行期/链接期符号声明**：需要为 ROM 输出表中引用到的 C 符号提供原型（至少保证编译期可见）。
3) **避免多 TU 重定义**：通过 `MQUICKJS_RIDL_DECLARE_ONLY` 抑制定义，避免多个翻译单元重复定义同一批静态/函数。

目前我们把 (2) 与 (1)(3) 绑定在同一个宏条件下：

- `MQUICKJS_RIDL_DECLARE_ONLY` 打开时，不仅抑制定义，也抑制了原型声明

这在 mquickjs-build 的构建流水线下是天然不匹配的：

- `mqjs_ridl_stdlib.h` 中的 `js_c_function_table` 与 class 表是由 host tool 生成的（需要 (1)）。
- `mqjs_stdlib_impl.c` 则在 `DECLARE_ONLY` 下编译（需要 (2) 但不要 (1)）。

因此这不是某个补丁点能长期稳定解决的问题，而是“生成物职责未分离”的结构性问题。

## 3. 设计目标

- G1：保证 ROM 输出头中引用到的所有 C 符号，在 `mqjs_stdlib_impl.c` 的编译单元内都**有可见原型**。
- G2：避免多翻译单元重复定义同一批静态表/函数。
- G3：不改变现有 mquickjs-build 的“DECL-only 编译策略”语义（继续用 `-DMQUICKJS_RIDL_DECLARE_ONLY` 控制定义抑制），但让其不会误杀原型。
- G4：为后续扩展（更多 builtins、更多 module、更多 app/profile）提供清晰可维护的边界：消费者不必理解复杂宏组合。

## 4. 方案比较

### 方案 A：继续单一 header，通过宏修补

做法：保留一个 `mquickjs_ridl_register.h`，引入更多宏：
- `MQUICKJS_RIDL_EMIT_DECLS`（默认开）
- `MQUICKJS_RIDL_EMIT_DEFS`（host tool 开，DECLARE_ONLY 关）

优点：文件数不变。
缺点：宏组合继续膨胀，后续新增编译形态（多 app、多 profile、额外工具）会再次踩坑。

### 方案 B：拆分 decl/def，并提供 umbrella 入口（推荐）

生成 3 个头文件：

- `mquickjs_ridl_register_decl.h`
  - 只包含 `extern` 与函数原型（永远不会产生定义）
  - 在任何 TU、任何宏条件下都可安全 include

- `mquickjs_ridl_register_def.h`
  - 包含 `static const JSPropDef[]`、`static const JSClassDef`、module ctor 的函数体等
  - 仅在 host tool 编译期需要

- `mquickjs_ridl_register.h`（umbrella 稳定入口）
  - 始终 `#include "mquickjs_ridl_register_decl.h"`
  - 在 `__HOST__` 且 **未** `MQUICKJS_RIDL_DECLARE_ONLY` 时，再 `#include "mquickjs_ridl_register_def.h"`

优点：职责边界清晰；后续需求增长可控；消费者只 include umbrella。
缺点：生成物文件数增加；mquickjs-build 需要拷贝多个头。

结论：选方案 B。

## 5. 具体设计（方案 B）

### 5.1 头文件内容划分

- decl：
  - `JSValue js_ridl_require(JSContext*, JSValue*, int, JSValue*);`
  - 所有 `js_*` glue 的原型（包括 module ctor 的原型）
  - `extern const JSClassDef js_*_class_def;`（对需要跨文件引用的 class_def）
  - `typedef`/`extern` 表项声明（如果需要被 require.c 等使用）

- def：
  - `static const JSPropDef ...`、`static const JSClassDef ...` 的定义
  - `JSValue js_*_module_constructor(...) { ... }` 的函数体（如仍选择 stub）
  - `JS_RIDL_DECLS` 这类供 `mqjs_stdlib_template.c` 产生 ROM 根可达图的静态结构

- umbrella：
  - 作为 mquickjs-build copy 的唯一稳定文件名

### 5.2 mquickjs-build 调整

mquickjs-build 当前只 copy 一个 `mquickjs_ridl_register.h` 到 include 目录。
需要改为：

- copy `mquickjs_ridl_register.h`（umbrella）
- copy `mquickjs_ridl_register_decl.h`
- copy `mquickjs_ridl_register_def.h`

并维持现有编译策略：
- 编译 host tool：`-D__HOST__`（且不定义 DECLARE_ONLY）→ umbrella 会 include def
- 编译 `mqjs_stdlib_impl.c`：`-DMQUICKJS_RIDL_DECLARE_ONLY -include mquickjs_ridl_register.h` → umbrella 只 include decl，不会 include def

### 5.3 `mqjs_stdlib_template.c` 与 `mqjs_stdlib_impl.c` 约定

- `mqjs_stdlib_template.c`：仅 include umbrella 即可（host tool 编译期会自动包含 def）。
- `mqjs_stdlib_impl.c`：不要手工 include register（mquickjs-build 已用 `-include` 强制注入）。保持 include `mqjs_ridl_stdlib.h`。

## 6. 与方案 D（__ridl_modules 命名空间对象）关系

方案 D 的目标：让 module class_def 在 ROM build 的“可达根”上出现，但不污染 global 顶层。
实现方式：在 def 部分生成一个 `JS_OBJECT_DEF("RidlModules", props)`，并在 global 扩展中导出 `__ridl_modules` 指向该对象。

头文件拆分不会改变 D 的语义，只是确保：
- ROM build 能看到 def（含该对象与 module class_def）
- runtime 编译时不会因为 DECLARE_ONLY 误杀原型而失败

## 7. 测试矩阵（必须通过）

1) `cargo run -p ridl-builder -- prepare --profile framework`
   - base build + ridl build 都应成功
2) `cargo test`
3) `cargo run -- tests`
4) JS 集成：`cargo run -- tests -- --filter global/require`

验收点：
- require 用例可见 `m1.ping` 与 `m1.Foo`（每次 require 返回新实例，无缓存）。

## 8. 风险与回滚策略

- 风险：mquickjs-build 的 copy 逻辑与 include 目录结构变更，可能影响下游 include 路径。
  - 缓解：umbrella 文件名保持不变；仅增加额外头文件。

- 风险：decl/def 划分不完整导致缺原型/缺定义。
  - 缓解：以 `mqjs_ridl_stdlib.h` 中出现的所有 `js_*` 符号为准，保证 decl 覆盖。

---

状态：草案（待确认后开始实现）
