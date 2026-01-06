# RIDL Modules

这个目录包含一系列用于测试RIDL（Rust Interface Description Language）功能模块的示例。

## 目录结构

- `stdlib/` - mquickjs标准库平台相关功能的RIDL定义和实现
  - `stdlib.ridl` - RIDL定义文件
  - `stdlib_impl.rs` - Rust实现代码
  - `stdlib_glue.rs` - Rust胶水代码实现
  - `stdlib_glue.h` - C头文件定义

## 设计说明

### stdlib/stdlib.ridl

定义了mquickjs标准库中的平台相关功能，这些功能通过全局注册方式注册到全局对象中，包括：

- 打印函数
- 垃圾回收函数
- 日期和性能相关函数
- 脚本加载函数
- 定时器相关函数
- 控制台单例对象

### 多模块支持架构

为了支持多个RIDL定义，我们采用以下架构：

1. 每个RIDL模块在`ridl_modules/`下有自己的子目录
2. 每个子目录包含该模块的`.ridl`定义文件和对应的Rust胶水代码和Rust实现代码
3. 所有模块的注册信息通过`mquickjs_ridl_register.h`统一集成到标准库中
4. `jidl-tool`将生成所有模块的C函数实现和注册定义
5. `mqjs_stdlib_template.c`通过`JS_RIDL_EXTENSIONS`宏包含所有模块的扩展

未来可能的模块示例：
- `network/` - 网络功能模块
- `file/` - 文件系统功能模块
- `crypto/` - 加密功能模块

### 实现方式

按照"复杂代码生成工具开发策略"，我们先手动实现生成代码，跑通整个流程，具体包括：

1. Rust胶水代码（stdlib_glue.rs）- 处理类型转换、函数绑定等底层细节
2. Rust实现（stdlib_impl.rs）- 与mquickjs-rs集成
3. 项目配置（Cargo.toml）- Rust项目依赖管理

## 构建系统集成

这个模块已经集成到项目的构建系统中：

1. `Cargo.toml` - 添加了构建依赖

## 使用方法

要测试这个功能，可以运行：
```bash
cargo run test.js
```

其中test.js可以包含对这些标准库功能的调用，例如：
```javascript
print("Hello from JS!");
console.log("Testing console.log");
console.error("Testing console.error");
let timerId = setTimeout(() => {
    print("Timer executed!");
}, 1000);
gc();  // 触发垃圾回收
let now = performance_now();
print("Current time:", now);
```

## 后续开发

在验证此手动实现的流程后，我们将开发jidl-tool来自动生成类似的胶水代码.