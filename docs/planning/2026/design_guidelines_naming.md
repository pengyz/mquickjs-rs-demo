<!-- planning-meta
status: 未复核
tags: types
replaced_by:
- docs/README.md
-->

> 状态：**未复核**（`types`）
>
> 现行口径/替代：
> docs/README.md
>
> 关键结论：
> - （待补充：3~5 条）
# 设计准则：命名、命名空间与符号（Rust vs C）

本文件用于沉淀一条通用设计准则：

- **Rust 侧类型/符号**：优先使用 *命名空间（Rust module）* 进行隔离与组织。
- **C 导出符号**：必须 *flatten*（使用 `_`）以满足 C 符号约束。

该准则适用于：
- generator 生成的 Rust 类型（struct/enum/trait helper 等）
- glue 生成的 Rust 函数/静态符号
- C ABI 导出符号（`extern "C"`）
- JS 可见符号（通常由 C 导出函数注册到 global/module）

## 1. Rust：用命名空间隔离，避免拼接长名字

### 1.1 原则

- 避免用 `_` 拼接成长类型名（例如把 scope/module/函数/参数全部 flatten 到一个标识符）。
- 改为：
  - 用一层或多层 `mod` 来承载域名、模块名、类型类别。
  - 在该命名空间内部使用短且语义明确的 PascalCase/CamelCase 类型名。

### 1.2 推荐结构（示例）

```rust
pub mod ridl_types {
    pub mod global {
        pub mod union {
            pub enum EchoStringOrInt {
                String(String),
                Int(i32),
            }
        }
    }
}
```

> 注意：本仓库约定全局域名使用 `global`（小写），模块域名使用 normalize 后的字符串。

## 2. module mode 预留：类型域（type domain）

生成类型/符号时必须携带“类型域”信息：

- 若为全局注册：域名固定为 `global`
- 若属于某 module：域名为 module path normalize 后的字符串

normalize 规则（与仓库既有约定对齐）：
- 将任何非 `[A-Za-z0-9_]` 的字符替换为 `_`

该域名用于：
- Rust 命名空间层级（如 `ridl_types::global::...` 或 `ridl_types::<module>::...`）
- C/JS 符号名中的 module_name（flatten 后参与拼接）

## 3. C：导出符号必须 flatten（使用 `_`）

### 3.1 原则

- C ABI 的导出符号必须是单个标识符，因此需要 flatten。
- 连接符统一使用单个 `_`（不使用 `__` 作为分隔）。

### 3.2 说明

- 这条规则是**通用的 C 符号约束**，不针对某个具体类型（例如 union）。
- Rust 侧不应为了迁就 C 符号而把类型名也 flatten。

## 4. 与具体特性文档的关系

- 各特性设计文档（例如 union）可以引用本准则，而不重复展开。
- 若某特性需要额外的命名细节（例如 enum variant 命名），应在特性文档中补充。
