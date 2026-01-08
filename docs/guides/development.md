# 开发指南

## 概述

本文档为开发者提供在 mquickjs-rs 项目中进行开发的详细指南，包括 RIDL 模块开发、构建流程和最佳实践。

## 环境准备

### 必需工具

- Rust 工具链 (nightly 或稳定版)
- Cargo 包管理器
- Git 版本控制工具
- C 编译器 (用于构建 QuickJS)

### 项目结构

```
mquickjs-demo/
├── Cargo.toml              # 项目配置文件
├── build.rs                # 构建脚本（调用 ridl-tool 生成代码）
├── src/
│   └── main.rs             # 主程序入口
├── generated/              # 生成产物（*_glue.rs、*_impl.rs、ridl_symbols.rs 等）
├── ridl_modules/           # RIDL 模块源码与生成物的源目录
│   ├── stdlib/
│   └── stdlib_demo/
├── deps/
│   ├── mquickjs/           # QuickJS 源码
│   ├── mquickjs-rs/        # Rust 绑定库
│   └── ridl-tool/          # RIDL 解析/校验/生成 CLI
├── docs/                   # 项目文档
└── doc/planning/           # 需求计划文档（每个需求一份计划）
```

## RIDL 模块开发

### 创建/接入新模块（当前流程示例）

> 现有构建链在 `build.rs` 中列出了固定的 RIDL 文件（stdlib、stdlib_demo）。如果新增模块，需要同步修改 `build.rs` 的收集列表、模板生成逻辑以及符号聚合，确保生成产物被复制到根目录与 `generated/` 中。

1. 在 `ridl_modules/<your_module>/` 下创建模块目录，并提供：
```
ridl_modules/<your_module>/
├── Cargo.toml           # 模块的 Cargo 配置（依赖 mquickjs-rs）
├── <your_module>.ridl   # RIDL 定义
├── <your_module>_glue.rs   # 生成的 Rust 胶水代码（由 ridl-tool 生成）
├── <your_module>_impl.rs   # 生成的 Rust 实现骨架（由 ridl-tool 生成）
└── src/
    └── lib.rs           # 可选，按需要导出模块
```
2. 在 `Cargo.toml`（模块内）声明依赖：
```toml
[dependencies]
mquickjs-rs = { path = "../../deps/mquickjs-rs" }
```
3. 在 `<your_module>.ridl` 中定义接口：
```ridl
js_my_function(value: string);
js_another_function(number: int);
```
4. 运行 `cargo build`（会触发 `build.rs` 调用 ridl-tool）生成 `*_glue.rs` / `*_impl.rs`，并复制到项目根与 `generated/`。
5. 在生成的 `<your_module>_impl.rs` 中补全具体实现。
6. 若新增模块，确保在聚合阶段被包含（`ridl_symbols.rs`、`mquickjs_ridl_register.h`）。

> 注意：当前示例模块为 `stdlib_demo`（位于 `ridl_modules/stdlib_demo/`），可参考其 `Cargo.toml` 与生成产物布局。

```rust
use mquickjs_rs::{JSContext, JSValue};

fn say_hello() -> String {
    "Hello, World!".to_string()
}

#[no_mangle]
pub extern "C" fn js_say_hello(ctx: *mut JSContext, argc: i32, argv: *mut JSValue) -> JSValue {
    // 类型转换和参数验证
    if argc != 0 {
        return JS_ThrowTypeError(ctx, "say_hello expects no arguments\0".as_ptr() as *const i8);
    }
    
    // 调用impl文件中的具体实现
    let result = say_hello();
    
    // 将Rust类型转换为JSValue并返回
    mquickjs_rs::JS_NewString(ctx, result.as_str().as_ptr() as *const i8)
}
```

### 模块注册

模块通过 `JS_InitModuleSTDLib` 函数自动注册到 JavaScript 环境中。开发者无需手动注册单个函数。

### 类型映射

RIDL 定义的类型会自动映射到 Rust 类型：

- `string` → `&str` 或 `String`
- `int` → `i32` 或 `i64`
- `float` → `f64`
- `bool` → `bool`
- `array<T>` → `Vec<T>`
- `map<K, V>` → `HashMap<K, V>`

## Rust胶水代码与实现代码职责分离

### Glue文件职责（如my_module_glue.rs）

生成的胶水代码文件（[my_module_glue.rs](file:///home/peng/workspace/mquickjs-demo/tests/ridl_tests/stdlib/src/lib.rs)）承担以下职责：

1. **接口桥接**：作为 JavaScript 与 Rust 之间的桥接层
2. **引擎兼容函数**：包含使用 `#[no_mangle` 和 `extern "C"` 标记的函数，这些函数直接暴露给JavaScript引擎（例如 `js_say_hello`）
3. **参数验证**：验证传入参数的数量和类型
4. **类型转换**：在 JavaScript 类型和 Rust 类型之间进行转换
5. **错误处理**：处理和传播 Rust 与 JavaScript 之间的错误和异常
6. **ABI兼容性**：使用 `#[no_mangle` 和 `extern "C"` 确保 C ABI 兼容性
7. **调用实现**：调用 `impl.rs` 中的具体业务逻辑实现

胶水代码的主要作用是处理 JavaScript 与 Rust 之间的接口细节，而不需要关心具体的业务逻辑。

### Impl文件职责（如my_module_impl.rs）

实现代码文件（`my_module_impl.rs`）承担以下职责：

1. **业务逻辑实现**：提供函数的具体业务逻辑实现
2. **功能实现**：包含实际功能函数的 Rust 实现（例如 `say_hello()`）
3. **算法实现**：实现具体的功能算法和数据处理
4. **业务规则**：实现具体的业务规则和处理流程
5. **Rust风格接口**：函数签名更符合Rust风格（例如 `fn say_hello() -> String`），不涉及JavaScript引擎的接口细节

实现文件不涉及接口桥接逻辑，专注于核心功能实现，通过被胶水代码调用来完成JavaScript调用的完整流程。

## 构建流程

### 构建 RIDL 模块

1. RIDL 工具解析 `.ridl` 文件
2. 生成 Rust 胶水代码
3. 编译模块为 rlib 库
4. 链接到主项目

### 构建主项目

运行以下命令构建项目：

```bash
cargo build
```

此命令将：
1. 执行 `build.rs` 脚本
2. 生成 RIDL 标准库扩展
3. 编译所有依赖
4. 链接最终的可执行文件

## 调试技巧

### 调试 RIDL 生成的代码

1. 检查生成的 [module_name_glue.rs](file:///home/peng/workspace/mquickjs-demo/tests/ridl_tests/stdlib/src/lib.rs) 文件内容
2. 确认函数签名是否正确
3. 验证参数转换逻辑

### 调试 JavaScript 与 Rust 交互

1. 在 Rust 实现中添加日志输出
2. 使用 `console.log` 在 JavaScript 中输出调试信息
3. 检查类型转换是否正确

### 构建错误排查

常见构建错误及解决方案：

1. "找不到 mquickjs-rs 依赖"：
   - 检查路径是否正确
   - 确认 [Cargo.toml](file:///home/peng/workspace/mquickjs-demo/Cargo.toml) 中的依赖声明

2. "函数符号未定义"：
   - 检查函数是否使用 `#[no_mangle` 和 `extern "C"` 标记
   - 确认函数名是否正确拼写

3. "类型转换错误"：
   - 检查 RIDL 定义和 Rust 实现之间的类型匹配
   - 确认参数数量和类型是否一致

## 最佳实践

### RIDL 设计

1. **保持接口简洁**：每个函数只做一件事
2. **使用描述性名称**：函数名和参数名应清晰表达其用途
3. **合理使用类型**：明确指定参数类型以提高安全性
4. **错误处理**：在 Rust 实现中正确处理错误情况

### Rust 实现

1. **内存安全**：遵循 Rust 的所有权规则
2. **错误传播**：使用 Result 类型处理可能的错误
3. **性能优化**：避免不必要的内存分配
4. **文档注释**：为公共函数提供文档注释

### 模块化构建

1. **独立测试**：每个模块应可独立测试
2. **清晰依赖**：明确定义模块间的依赖关系
3. **版本管理**：为模块指定适当的版本号
4. **向后兼容**：API 变更时保持向后兼容性

## 与 C 胶水代码方案的对比

### 旧方案（C 胶水代码）

- 生成 C 语言胶水代码
- 需要 C 编译器参与构建
- 类型转换在 C 代码中处理
- 需要额外的头文件管理
- 需要手动处理内存管理

### 新方案（Rust 胶水代码）

- 生成 Rust 语言胶水代码
- 统一使用 Rust 工具链构建
- 类型转换在 Rust 代码中处理
- 更好的内存安全保证
- 与 Rust 生态更好的集成
- 更简单的依赖管理
- 更好的错误处理机制

## 相关文档

- [RIDL 语法与扩展](../ridl/syntax-and-extension.md) - RIDL 语言的语法定义和规范
- [标准库扩展机制](../ridl/stdlib-extension-mechanism.md) - 标准库扩展的实现机制和流程
- [Rust胶水代码演进](../ridl/rust-glue-evolution.md) - 从C胶水代码到Rust胶水代码的演进过程
- [RIDL 模块设计](../ridl/module-design.md) - RIDL 模块的设计和实现细节

## 测试策略

### 单元测试

为每个 RIDL 模块编写单元测试，验证：

1. 函数功能是否正确
2. 参数验证是否有效
3. 错误处理是否恰当
4. 边界情况是否处理

### 集成测试

编写集成测试验证：

1. 模块与 JavaScript 的交互
2. 跨模块功能
3. 性能指标
4. 内存使用情况

### 性能测试

定期运行性能测试，确保：

1. 函数调用开销在可接受范围内
2. 内存使用没有泄漏
3. 并发访问安全

## 贡献指南

### 提交代码

1. Fork 仓库
2. 创建功能分支
3. 编写代码并添加测试
4. 提交 PR 并描述变更内容

### 代码审查

提交的代码需要通过以下审查：

1. 代码风格符合项目规范
2. 测试覆盖率满足要求
3. 性能指标达标
4. 文档更新完整

## 常见问题

### RIDL 语法问题

1. **语法错误**：检查 RIDL 语法是否符合规范
2. **类型错误**：确认类型定义是否正确
3. **函数重名**：避免在同一模块中定义重名函数

### 构建问题

1. **路径错误**：检查所有文件路径是否正确
2. **依赖错误**：确认所有依赖项已正确声明
3. **版本冲突**：解决依赖版本冲突问题

### 运行时问题

1. **函数未找到**：检查函数是否正确注册
2. **类型不匹配**：确认 JS 与 Rust 间类型转换正确
3. **内存错误**：检查内存管理和所有权问题