---
name: ridl-global-function-architecture
description: RIDL 全局函数的代码生成架构——glue 生成 C FFI，用户在 impls 提供实现，api.rs 不生成
type: pattern
created: 2026-09-04
sources: [deps/ridl-tool/templates/rust_glue.rs.j2]
---

## RIDL 全局函数架构

**RIDL 定义**：`fn helper(x: i32, y: i32) -> i32;`

**生成流程**：
1. `glue.rs` 生成 `extern "C" fn js_helper(ctx, ...)` — C FFI 包装
2. 包装内部调用 `helper(x, y)` — 用户实现
3. 用户在 `crate::impls` 模块提供 `helper` 实现

**api.rs 不生成函数声明**：
- 函数没有 trait 抽象（不像 class/singleton）
- 生成无实现体的 `fn helper(...);` 会导致 Rust 编译错误
- 用户直接在 `impls` 模块实现即可

**与 class/singleton 的区别**：
- class/singleton：api.rs 生成 trait → 用户实现 trait → glue 调用 trait 方法
- global function：api.rs 不生成 → 用户直接在 impls 实现 → glue 调用

**示例**：
```rust
// 用户代码：crate::impls 模块
pub fn helper(x: i32, y: i32) -> i32 {
    x + y
}
```
