# RIDL 模块设计与实现

## 概述

RIDL (Rust Interface Definition Language) 模块是 mquickjs-rs 项目中的核心组件，用于定义 JavaScript 和 Rust 之间的接口。每个 RIDL 模块都作为独立的 Rust crate 存在，遵循模块化构建的设计原则。

## RIDL 模块架构

### 模块结构

每个 RIDL 模块应包含以下文件和目录：

```
module_name/
├── Cargo.toml           # 模块的 Cargo 配置文件
├── module_name.ridl     # RIDL 定义文件
├── module_name_glue.rs  # Rust 胶水代码（由 RIDL 工具生成）
├── module_name_impl.rs  # Rust 实现文件
└── src/
    └── lib.rs           # 模块入口（可选，取决于实现方式）
```

### 模块作为独立 Crate

根据架构规范，每个 RIDL 模块都应作为独立的 Rust crate 存在，具有以下特点：

1. **独立的 Cargo.toml 文件**：定义模块的依赖关系和构建配置
2. **独立的构建单元**：可以独立编译成 rlib 或静态库
3. **模块化依赖管理**：通过 Cargo 系统管理模块间的依赖关系

## RIDL 模块实现细节

### 1. RIDL 定义文件

RIDL 定义文件（`.ridl` 扩展名）定义了 JavaScript 可调用的函数接口：

```ridl
// ridl_module_demo_default.ridl 示例
fn default_echo_str(s: string) -> string;
```

### 2. Rust 胶水代码生成

RIDL 工具会根据 RIDL 定义文件生成 Rust 胶水代码（例如 `<module>_glue.rs`），其主要作用包括：

- 定义从 JavaScript 到 Rust 的函数桥接
- 处理参数类型转换
- 调用实际的 Rust 实现函数
- 处理错误和异常情况

生成的胶水代码遵循 C ABI，以确保与 JavaScript 引擎的兼容性。

### 3. Rust 实现

Rust 实现文件（如 `module_name_impl.rs`）提供函数的具体业务逻辑实现，使用标准 Rust 接口：

``rust
fn say_hello() -> String {
    "Hello, World!".to_string()
}
```

而引擎兼容的函数接口则在 `module_name_glue.rs` 中定义：

```rust
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

## Rust胶水代码与实现代码职责分离

### Glue文件职责（如module_name_glue.rs）

生成的胶水代码文件（例如 `<module>_glue.rs`）承担以下职责：

1. **接口桥接**：作为 JavaScript 与 Rust 之间的桥接层
2. **引擎兼容函数**：包含使用 `#[no_mangle` 和 `extern "C"` 标记的函数，这些函数直接暴露给JavaScript引擎（例如 `js_say_hello`）
3. **参数验证**：验证传入参数的数量和类型
4. **类型转换**：在 JavaScript 类型和 Rust 类型之间进行转换
5. **错误处理**：处理和传播 Rust 与 JavaScript 之间的错误和异常
6. **ABI兼容性**：使用 `#[no_mangle` 和 `extern "C"` 确保 C ABI 兼容性
7. **调用实现**：调用 `impl.rs` 中的具体业务逻辑实现

胶水代码的主要作用是处理 JavaScript 与 Rust 之间的接口细节，而不需要关心具体的业务逻辑。

### Impl文件职责（如module_name_impl.rs）

实现代码文件（`module_name_impl.rs`）承担以下职责：

1. **业务逻辑实现**：提供函数的具体业务逻辑实现
2. **功能实现**：包含实际功能函数的 Rust 实现（例如 `say_hello()`）
3. **算法实现**：实现具体的功能算法和数据处理
4. **业务规则**：实现具体的业务规则和处理流程
5. **Rust风格接口**：函数签名更符合Rust风格（例如 `fn say_hello() -> String`），不涉及JavaScript引擎的接口细节

实现文件不涉及接口桥接逻辑，专注于核心功能实现，通过被胶水代码调用来完成JavaScript调用的完整流程。

## 模块构建流程

### 1. RIDL 工具处理

1. RIDL 工具解析 `.ridl` 文件
2. 生成 Rust 胶水代码
3. 生成注册代码

### 2. 编译阶段

1. 编译 RIDL 模块为独立的 Rust crate
2. 链接到主程序

### 3. 注册阶段

在运行时，通过 `JS_InitModuleSTDLib` 函数将模块注册到 JavaScript 环境中。

## 模块化构建的优势

1. **独立开发**：不同模块可以独立开发和测试
2. **可扩展性**：可以轻松添加新模块而无需修改核心代码
3. **可维护性**：模块间解耦，便于维护和升级
4. **重用性**：模块可以在不同项目间重用

## 最佳实践

### 模块命名规范

- 模块名应使用小写字母和下划线
- 与对应的 `.ridl` 文件名保持一致
- 避免使用保留字和特殊字符

### 接口设计原则

- 函数命名应清晰、简洁
- 参数和返回值类型应明确
- 错误处理应统一

### 依赖管理

- 模块应明确声明其依赖
- 避免循环依赖
- 优先使用项目内部依赖而非外部依赖

## 与C胶水代码方案的对比

### 旧方案（C胶水代码）

- 生成 C 语言胶水代码
- 需要 C 编译器参与构建
- 类型转换在 C 代码中处理
- 需要额外的头文件管理

### 新方案（Rust胶水代码）

- 生成 Rust 语言胶水代码
- 统一使用 Rust 工具链构建
- 类型转换在 Rust 代码中处理
- 更好的内存安全保证
- 更好的 Rust 生态集成

## 相关文档

- [RIDL 语法与扩展](syntax-and-extension.md) - RIDL 语言的语法定义和规范
- [标准库扩展机制（已过时）](../legacy/stdlib-extension-mechanism.md) - 历史机制记录；现行口径见 `docs/build/pipeline.md` / `docs/ridl/codegen-outputs.md`
- [Rust胶水代码演进（历史/部分过时）](../legacy/rust-glue-evolution.md) - 演进记录（不作为现行规范）

## 未来发展方向

### 1. 动态模块加载

> 备注：当前 mquickjs 的 stdlib 扩展注册必须发生在编译期（C 侧包含 `mquickjs_ridl_register.h` 并在 stdlib 表处展开），因此**无法在运行时动态注册**标准库扩展项。
> 如果未来要支持“动态加载”，需要区分：
> - JS 层面的模块加载（`require`/module system）
> - C 侧 stdlib 扩展表的编译期注册（不可运行时变更）

### 2. 模块版本管理

引入模块版本管理机制，支持不同版本模块的共存和兼容性检查。

### 3. 模块市场

构建 RIDL 模块市场，提供预构建的常用模块，方便开发者使用。