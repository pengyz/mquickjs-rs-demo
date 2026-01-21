# mquickjs-rs 项目架构概述

## 项目概述

mquickjs-rs 是一个基于 QuickJS 的 Rust 绑定库，旨在提供安全、高效的 JavaScript 执行环境。该项目采用模块化设计，通过 RIDL (Rust Interface Definition Language) 实现 JavaScript 与 Rust 之间的接口定义和绑定。

## 核心架构

### 整体架构

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   JavaScript    │◄──►│   mquickjs-rs    │◄──►│     Rust        │
│   Runtime       │    │   Bindings       │    │   Functions     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              ▲
                              │
                       ┌──────────────────┐
                       │   RIDL 工具      │
                       │   (Code Gen)     │
                       └──────────────────┘
```

### 组件构成

1. **QuickJS 引擎**: 轻量级 JavaScript 引擎
2. **Rust 绑定层**: 提供安全的 Rust 接口
3. **RIDL 工具**: 接口定义语言和代码生成工具
4. **模块系统**: 支持可扩展的功能模块

## RIDL 模块系统

### 模块化设计原则

mquickjs-rs 采用模块化设计，每个功能模块都作为独立的 Rust crate 存在。这种设计带来以下优势：

- **可扩展性**: 可以轻松添加新功能模块
- **可维护性**: 模块间解耦，便于维护
- **重用性**: 模块可以在不同项目间重用
- **独立开发**: 不同模块可以独立开发和测试

### 模块结构（当前仓库）

每个 RIDL 模块位于 `ridl-modules/<module>/`，是一个独立 Rust crate，包含：

1. **RIDL 定义文件**（`src/*.ridl`）：定义 JavaScript 可调用的接口
2. **Rust 代码**（`src/lib.rs` / `src/impls.rs` 等）：提供 Rust 侧实现与初始化入口
3. **构建配置**（`Cargo.toml`）：模块的 Cargo 配置

生成产物不再落在仓库内固定目录；由 **App `build.rs`** 在构建时生成到 `$OUT_DIR/`（例如 `ridl_bootstrap.rs`、`mquickjs_ridl_register.h`），并由 `mquickjs-sys`/`mquickjs-rs` 在编译期引用。

### Rust胶水代码与实现代码职责分离

#### Glue文件职责（如 `<module>_glue.rs`）

生成的胶水代码文件（例如 `<module>_glue.rs`）承担以下职责：

1. **接口桥接**：作为 JavaScript 与 Rust 之间的桥接层
2. **引擎兼容函数**：包含使用 `#[no_mangle]` 和 `extern "C"` 标记的函数，这些函数直接暴露给JavaScript引擎（例如 `js_say_hello`）
3. **参数验证**：验证传入参数的数量和类型
4. **类型转换**：在 JavaScript 类型和 Rust 类型之间进行转换
5. **错误处理**：处理和传播 Rust 与 JavaScript 之间的错误和异常
6. **ABI兼容性**：使用 `#[no_mangle]` 和 `extern "C"` 确保 C ABI 兼容性
7. **调用实现**：调用 `impl.rs` 中的具体业务逻辑实现

胶水代码的主要作用是处理 JavaScript 与 Rust 之间的接口细节，而不需要关心具体的业务逻辑。

#### Impl文件职责（如module_name_impl.rs）

实现代码文件（`module_name_impl.rs`）承担以下职责：

1. **业务逻辑实现**：提供函数的具体业务逻辑实现
2. **功能实现**：包含实际功能函数的 Rust 实现（例如 `say_hello()`）
3. **算法实现**：实现具体的功能算法和数据处理
4. **业务规则**：实现具体的业务规则和处理流程
5. **Rust风格接口**：函数签名更符合Rust风格（例如 `fn say_hello() -> String`），不涉及JavaScript引擎的接口细节

实现文件不涉及接口桥接逻辑，专注于核心功能实现，通过被胶水代码调用来完成JavaScript调用的完整流程。

### 模块注册机制

模块通过 `JS_InitModuleSTDLib` 函数注册到 JavaScript 环境中，使定义的函数在 JS 中可用。

## 构建系统

### 构建流程（Plan B：编译期注册）

> mquickjs 仅支持编译期将扩展注册进 stdlib 表，因此 RIDL 扩展的选择与聚合发生在构建期。

1. **选择 registry source（app manifest）**：由 profile 决定（见 `mquickjs.build.toml`）
2. **RIDL 解析/聚合**：`ridl-tool resolve/generate` 生成 `mquickjs_ridl_register.h`（用于 C 侧编译期 stdlib 注入）
3. **编译 C 静态库**：`mquickjs-build` 将扩展编译进 `libmquickjs.a`
4. **bindgen**：生成 Rust FFI bindings（Rust 2024 输出）

### 依赖管理

- 使用 Cargo 管理 Rust 依赖
- 模块间通过标准的 Rust 依赖机制管理
- 支持本地路径和远程仓库依赖

## 安全性设计

### 内存安全

- 利用 Rust 的所有权系统确保内存安全
- 避免常见的内存错误（如悬空指针、缓冲区溢出等）

### 执行安全

- JavaScript 代码在沙箱环境中执行
- 限制对系统资源的直接访问

## 扩展性设计

### 模块扩展

通过 RIDL 模块系统，可以轻松添加新功能：

1. 定义新的 RIDL 接口
2. 实现对应的 Rust 函数
3. 通过构建系统自动集成

### API 扩展

- 提供扩展 API 以支持自定义功能
- 支持插件化架构

## 性能优化

### 运行时性能

- 最小化 JS 与 Rust 间的数据转换
- 优化函数调用路径
- 高效的内存管理

### 构建性能

- 增量编译支持
- 并行构建优化

## 开发流程

### 模块开发

1. 定义 RIDL 接口
2. 实现 Rust 功能
3. 测试模块功能
4. 集成到主系统

### 测试策略

- 单元测试覆盖各模块功能
- 集成测试验证整体功能
- 性能测试确保运行效率

## 相关文档

- [RIDL 语法与扩展](../ridl/syntax-and-extension.md)：RIDL 语言的语法定义和规范
- [标准库扩展机制（已过时）](../legacy/stdlib-extension-mechanism.md)：历史机制记录；现行口径见 `../build/pipeline.md` / `../ridl/codegen-outputs.md`
- [RIDL 模块设计](../ridl/module-design.md)：RIDL 模块的设计和实现细节

## 未来发展方向

### 动态模块加载

计划支持运行时动态加载 RIDL 模块，提升系统灵活性。

### 模块市场

构建 RIDL 模块市场，提供预构建的常用模块。

### 高级语言特性

支持更高级的类型系统和错误处理机制。