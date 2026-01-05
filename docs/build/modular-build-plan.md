# 模块化构建实施计划与架构规范

## 概述

本文档详细说明了 mquickjs-rs 项目的模块化构建计划。该计划旨在实现 RIDL 模块的独立开发、构建和集成，以提高项目的可维护性和扩展性。

## 设计目标

1.  **模块独立性**：每个 RIDL 模块应能够独立开发和测试
2.  **构建解耦**：模块的构建过程不应相互依赖
3.  **易于扩展**：可以轻松添加新的 RIDL 模块
4.  **统一接口**：所有模块使用统一的接口标准
5.  **Rust 胶水代码**：使用 Rust 胶水代码替代 C 胶水代码，提高内存安全

## 模块结构

### 标准模块结构

每个 RIDL 模块应遵循以下目录结构：

```
module_name/
├── Cargo.toml           # 模块的 Cargo 配置文件
├── module_name.ridl     # RIDL 定义文件
├── module_name_glue.rs  # Rust 胶水代码（由 RIDL 工具生成）
├── module_name_impl.rs  # Rust 实现文件
└── src/
    └── lib.rs           # 模块入口（可选，取决于实现方式）
```

### Cargo.toml 配置

每个模块的 Cargo.toml 应包含以下配置：

```toml
[package]
name = "module_name"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib", "staticlib"]

[dependencies]
mquickjs-rs = { path = "../../../deps/mquickjs-rs" }
```

## 构建规范

1.  每个 RIDL 模块（如 stdlib、stdlib_demo）必须作为独立 Rust crate 存在，拥有独立 Cargo.toml，编译生成 rlib 静态库。
2.  rlib 库包含该模块的 Rust 实现代码 (`*_impl.rs`) 及 RIDL 工具生成的 Rust 胶水代码 (`*_glue.rs`)。
3.  mquickjs-rs 主库不直接编译 RIDL 模块源码，而是通过链接基础 mquickjs.a 库与各 RIDL 模块生成的 rlib 库完成最终构建。
4.  禁止在 mquickjs-rs 的 build.rs 中直接处理 RIDL 模块的胶水代码编译。
5.  禁止将 RIDL 扩展相关的 C 胶水代码（如 `mqjs_stdlib_impl.c`）复制到上游 `mquickjs` 项目中，所有扩展文件保留在 `mquickjs-rs` 项目内。
6.  RIDL 模块的编译和链接由使用者（如 mquickjs-demo）在 build.rs 中处理，不归属于 mquickjs-rs 内部构建流程。
7.  各模块生成的 rlib 库由最终使用者在构建时统一链接。

## 编译流程

1.  使用 ridl-tool 编译所有 ridl 接口文件，生成标准库注册代码（如 mqjs_stdlib.h 和 mquickjs_atoms.h）。
2.  通过 mquickjs-rs 的 build.rs 将注册代码构建成 mqjs_ridl_stdlib 工具，并编译 mquickjs.a 库，整合 mquickjs.c 主体代码与生成的标准库头文件。
3.  使用 bindgen 从 C 头文件生成 Rust FFI 绑定。
4.  RIDL 工具解析 `.ridl` 文件并生成相应的 Rust 胶水代码 (`*_glue.rs`)。
5.  每个 RIDL 模块作为独立的 Rust crate 进行编译，生成 rlib 库。
6.  mquickjs-rs 链接 mquickjs.a 库及各 RIDL 模块生成的 rlib 库，形成带标准库扩展的 Rust binding 库。
7.  mquickjs-demo 项目链接 mquickjs-rs，最终生成可执行 JS 解释器。

## 架构原则

-   每个 RIDL 模块为独立 crate，编译为 rlib 静态库。
-   mquickjs-rs 不直接编译 RIDL 模块源码，仅通过链接已编译的 rlib 库完成集成。
-   禁止修改上游 mquickjs.h 等原始头文件，应通过构建脚本处理依赖。
-   标准库功能通过静态注册方式集成到 JavaScript 引擎中。

## 注意事项

-   直接运行 `mqjs_ridl_stdlib` 工具可能无法生成包含最新扩展的头文件，建议始终使用 `cargo build` 触发完整构建流程。
-   头文件路径应使用绝对路径或基于 crate 根目录的相对路径，避免路径解析错误。

## RIDL 工具集成

### RIDL 工具职责

1.  解析 RIDL 语法文件。
2.  生成 Rust 胶水代码 (`module_name_glue.rs`)。
3.  生成类型转换和错误处理代码。
4.  生成模块注册代码。

### 代码生成规则

-   所有生成的 Rust 函数必须使用 `#[no_mangle` 和 `extern "C"` 标记。
-   生成的代码必须遵循 C ABI 以确保与 JavaScript 引擎的兼容性。
-   参数验证和类型转换必须在胶水代码中处理。
-   错误处理应遵循 Rust 和 JavaScript 的错误传播机制。

## 模块注册机制

### 注册流程

1.  每个 RIDL 模块实现标准的初始化函数。
2.  初始化函数将模块中的函数注册到 JavaScript 环境。
3.  mquickjs-rs 在运行时调用各模块的初始化函数。

### 标准接口

所有 RIDL 模块必须实现以下标准接口：

```rust
// 模块初始化函数
pub extern "C" fn js_init_module(module_name: *mut JSContext) -> *mut JSModuleDef;
```

## 依赖管理

### 内部依赖

-   RIDL 模块依赖 mquickjs-rs 提供的基础 API。
-   模块间依赖应尽量避免，如必须使用则通过标准接口通信。

### 版本管理

-   每个模块独立版本管理。
-   主项目指定所依赖模块的版本范围。
-   支持模块的向后兼容性检查。

## 测试策略

### 模块测试

1.  每个 RIDL 模块应有独立的单元测试。
2.  测试应覆盖所有定义的函数。
3.  集成测试验证模块与 JavaScript 的交互。

### 集成测试

1.  测试模块间的交互。
2.  验证模块注册机制。
3.  性能测试确保无内存泄漏。

## 与 C 胶水代码方案的对比

### 旧方案（C 胶水代码）

-   生成 C 语言胶水代码。
-   需要 C 编译器参与构建。
-   类型转换在 C 代码中处理。
-   需要额外的头文件管理。
-   需要手动处理内存管理。

### 新方案（Rust 胶水代码）

-   生成 Rust 语言胶水代码。
-   统一使用 Rust 工具链构建。
-   类型转换在 Rust 代码中处理。
-   更好的内存安全保证。
-   与 Rust 生态更好的集成。
-   更简单的依赖管理。
-   更好的错误处理机制。

## 实施步骤

### 步骤1：完善RIDL模块crate结构

为每个RIDL模块创建完整的crate结构：

```
tests/ridl_tests/stdlib/
├── Cargo.toml
├── stdlib.ridl
├── stdlib_glue.rs  # (由RIDL工具生成)
├── stdlib_impl.rs
└── src/
    └── lib.rs

tests/ridl_tests/stdlib_demo/
├── Cargo.toml
├── stdlib_demo.ridl
├── stdlib_demo_glue.rs  # (由RIDL工具生成)
├── stdlib_demo_impl.rs
└── src/
    └── lib.rs
```

### 步骤2：移除RIDL模块的build.rs

由于使用Rust胶水代码，不再需要为RIDL模块编写build.rs来编译C代码。因此，**删除** `tests/ridl_tests/stdlib/build.rs` 和 `tests/ridl_tests/stdlib_demo/build.rs` 文件。

### 步骤3：更新RIDL模块的Cargo.toml

更新每个RIDL模块的依赖，移除`[build-dependencies]`部分：

```toml
[package]
name = "stdlib"  # 或 stdlib_demo
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[dependencies]
mquickjs-rs = { path = "../../deps/mquickjs-rs" }
```

### 步骤4：更新mquickjs-rs的Rust代码

修改mquickjs-rs的Rust代码，使其通过依赖的RIDL模块来提供功能：

```rust
// 在mquickjs-rs的lib.rs中
use std::ffi::{CString, CStr};
use std::ptr;
use std::os::raw::c_char;

// 依赖于RIDL模块
use stdlib;
use stdlib_demo;

pub mod mquickjs_ffi;

pub struct Mquickjs {
    ctx: *mut std::os::raw::c_void,
}

// ... 其他实现 ...
```

## 验证步骤

1.  验证每个RIDL模块可以独立编译。
2.  验证mquickjs-rs可以链接所有模块。
3.  验证mquickjs-demo可以正常运行包含RIDL模块的功能。
4.  验证各模块的独立测试可以正常运行。

## 风险与缓解

-   **风险**：链接时可能出现符号冲突
    -   **缓解**：确保各模块使用唯一的符号名称。
-   **风险**：构建时间可能增加
    -   **缓解**：利用Rust的增量编译优化。
-   **风险**：依赖管理可能变得复杂
    -   **缓解**：使用workspace统一管理依赖。

## 相关文档

- [RIDL 语法与扩展](../ridl/syntax-and-extension.md) - RIDL 语言的语法定义和规范
- [标准库扩展机制](../ridl/stdlib-extension-mechanism.md) - 标准库扩展的实现机制和流程
- [Rust胶水代码演进](../ridl/rust-glue-evolution.md) - 从C胶水代码到Rust胶水代码的演进过程
- [RIDL 模块设计](../ridl/module-design.md) - RIDL 模块的设计和实现细节
- [开发指南](../guides/development.md) - 开发者指南，包括 RIDL 模块开发、构建流程和最佳实践