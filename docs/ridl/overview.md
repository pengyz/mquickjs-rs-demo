# RIDL 语义总览（现行口径）

> 本文档是 RIDL 在本仓库中的**现行语义口径**（Source of Truth）。
> 
> 规划/讨论过程见：`docs/planning/`。

## 1. 术语

- **RIDL module**：一个 crate，且其依赖路径的 `src/` 目录下至少包含一个 `*.ridl` 文件；否则不参与 registry 驱动的聚合。
- **GLOBAL**：未声明 `module ...` 的 RIDL 文件默认处于 GLOBAL 命名空间。
- **module 模式**：RIDL 文件声明了 `module ...` 时，产物应作为 `require("...")` 的模块导出，不得污染 `globalThis`。

## 2. 属性/字段语义（JS 侧）

### 2.1 `var/const`

- `var/const` 是 **JS-only 的 instance own data property**（每个实例一份）。
- `var/const` 必须显式初始化为 literal（生成期强约束）。
- 引擎不支持收紧属性描述符（`writable/enumerable/configurable` 无法全部精确表达）。
  - 约定：默认都为 true；`const` 仅通过 `writable=false` 的方式体现“不可写”（若实现路径可行）。

> 注意：`const` 的“只读效果”来自 JS property 层（如无 setter / writable=false），并不代表 Rust 侧语义变化。

### 2.2 `proto var`

- `proto var` 只挂在 **prototype** 上（不在实例 own props 上）。
- `proto var` 的安装/初始化走统一的 slot 机制，与 singleton 初始化时机一致。

## 3. 初始化与 correctness gate

- **唯一 correctness gate：`ridl_context_init(ctx)`**
  - RIDL 扩展的正确性（ctx-ext 分配、slot 初始化、proto var 安装等）必须通过该入口完成。
  - 不依赖任何“proto-ready hook”。

## 4. require 语义（与类导出）

- require 保留：**ROMClass → ctor materialize + DefineProperty 写回替换**。
- 行为：当模块对象的导出属性值为 ROMClass 时，`require()` 会将其 materialize 为 constructor function，并把 ctor 写回到模块对象属性。

## 5. module 模式与 globalThis

- module 模式下不得把 ctor/singleton 注入 `globalThis`。
- 任何需要安装在 prototype 上的 JS-only 字段（如 proto var）不得通过“globalThis 查找 ctor”实现。

## 6. 相关实现入口（代码）

- Rust 侧：`src/context.rs` 调用 `crate::ridl_context_ext::ridl_context_init(...)`
- 生成物：`$OUT_DIR/ridl_context_ext.rs`（包含 ctx-ext/slot/ridl_context_init）
- require：`deps/mquickjs-rs/require.c`（materialize + writeback）
