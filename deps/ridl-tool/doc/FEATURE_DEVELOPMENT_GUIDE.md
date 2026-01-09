# Feature模块开发指南

## 概述

本文档详细介绍了如何开发和集成基于RIDL的Feature模块。Feature模块是指实现特定功能的独立Cargo工程，它们通过ridl-tool生成绑定代码，并通过mquickjs提供JavaScript接口。

## 项目结构

### 标准Feature模块结构

```
features/network/
├── Cargo.toml
├── src/
│   ├── lib.rs          # 模块实现
│   └── api.rs          # API实现
├── network.ridl        # RIDL定义
└── README.md
```

### Workspace结构

```
mquickjs-rs/
├── Cargo.toml          # workspace定义
├── deps/
│   ├── ridl-tool/     # RIDL工具
│   ├── mquickjs-rs/   # Rust绑定
│   └── mquickjs/      # C引擎
├── features/
│   ├── network/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── network.ridl
│   ├── deviceinfo/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── deviceinfo.ridl
│   └── ui/
│       ├── Cargo.toml
│       ├── src/
│       └── ui.ridl
└── target/
```

## 创建新Feature模块

### 1. 创建Cargo工程

首先创建一个新的Cargo库工程：

```bash
cargo new --lib features/my_feature
cd features/my_feature
```

### 2. 编辑Cargo.toml

``toml
[package]
name = "my_feature"
version = "0.1.0"
edition = "2021"

[dependencies]
mquickjs = { path = "../../deps/mquickjs-rs" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 3. 创建RIDL定义文件

创建`my_feature.ridl`文件，定义JavaScript接口：

```
// 模块化接口定义
module system.my_feature@1.0

interface MyFeature {
    fn doSomething(input: string) -> string;
    fn processAsync(data: string, callback: callback(result: string, success: bool));
    fn getStatus() -> map<string, string>;
}
```

### 4. 实现Rust代码

在`src/lib.rs`中：

```rust
use mquickjs::{Context, JSValue, Result, AsyncCallback};

// 生成的trait，需要实现
pub trait MyFeature {
    fn do_something(&self, input: String) -> Result<String>;
    fn process_async(&self, data: String, callback: AsyncCallback) -> Result<()>;
    fn get_status(&self) -> Result<std::collections::HashMap<String, String>>;
}

// 实现具体的业务逻辑
pub struct MyFeatureImpl;

impl MyFeature for MyFeatureImpl {
    fn do_something(&self, input: String) -> Result<String> {
        Ok(format!("Processed: {}", input))
    }

    fn process_async(&self, data: String, callback: AsyncCallback) -> Result<()> {
        tokio::spawn(async move {
            // 模拟异步处理
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let result = format!("Async processed: {}", data);
            callback.call_with_args((result, true)).unwrap();
        });
        Ok(())
    }

    fn get_status(&self) -> Result<std::collections::HashMap<String, String>> {
        let mut status = std::collections::HashMap::new();
        status.insert("status".to_string(), "ok".to_string());
        status.insert("version".to_string(), "1.0".to_string());
        Ok(status)
    }
}

// 初始化函数
pub fn init_module(ctx: &Context) -> Result<()> {
    // 这里会由ridl-tool生成的代码调用
    Ok(())
}
```

### 5. 更新Workspace

在根目录的`Cargo.toml`中添加新模块：

``toml
[workspace]
members = [
    "deps/ridl-tool",
    "deps/mquickjs-rs",
    "features/network",
    "features/deviceinfo",
    "features/my_feature",  # 添加新模块
]
```

## RIDL语法规范

### 模块声明

```
// 模块声明格式
module <domain>.<feature_name>[@version]

// 示例
module system.network@1.0
module ui.components@2.1
module app.datastore
```

### 接口定义

```
interface FeatureName {
    // 同步方法
    fn syncMethod(param: string) -> string;
    
    // 异步方法（使用回调）
    fn asyncMethod(data: string, callback: callback(result: string, success: bool));
    
    // 无返回值方法
    fn actionMethod(param: int);
    
    // 复杂类型参数和返回值
    fn complexMethod(obj: MyStruct) -> map<string, array<int>>;
}
```

### 结构体定义

```
// JSON序列化结构体
json struct MyStruct {
    name: string;
    value: int;
    metadata: map<string, string>?;
}

// MessagePack序列化结构体
msgpack struct Config {
    settings: map<string, string>;
    enabled: bool;
}
```

## 构建流程

### 1. 代码生成

运行ridl-tool收集所有RIDL文件并生成绑定代码：

```bash
# 收集并生成所有绑定代码
cargo run -p ridl-tool -- --generate-all

# 或者使用make命令（如果配置了）
make generate-bindings
```

### 2. 编译项目

```
# 编译整个workspace
cargo build

# 或者只编译特定模块
cargo build -p my_feature
```

### 3. 自动化构建脚本

创建`build_features.sh`：

```
#!/bin/bash
set -e

echo "Step 1: Generate bindings from all .ridl files..."
cargo run -p ridl-tool -- --generate-all

echo "Step 2: Build the project..."
cargo build

echo "Build completed successfully!"
```

## 模块注册与访问

### 全局注册vs模块化注册

根据RIDL定义中的`module`声明，模块会被以不同方式注册：

1. **全局注册**（无module声明）：
   - 函数直接注册到global对象
   - 对象作为属性注册到global上（如console）

```
// 全局函数示例
fn setTimeout(callback: callback, delay: int);

// 定义一个网络接口
interface Network {
    fn connect(url: string) -> bool;
    fn disconnect();
    fn getStatus() -> object;
}

// 定义一个数据处理类
class DataProcessor {
    cache: map<string, string>?;
    DataProcessor();
    fn process(data: string) -> string;
    fn clearCache();
}
```

2. **模块化注册**（有module声明）：
   - 通过`require("module.name")`访问

```
// 模块化接口
module system.network@1.0
interface Network {
    fn getStatus() -> string;
}
```

### JavaScript端使用

```
// 使用模块化功能
var network = require("system.network");
var status = network.getStatus();

// 使用全局功能
console.log("Hello from console");
setTimeout(() => {
    console.log("Delayed message");
}, 1000);
```

## 最佳实践

### 1. 版本管理

- 为模块指定版本号，便于向后兼容
- 遵循语义化版本规范（MAJOR.MINOR.PATCH）

### 2. 错误处理

- 在异步操作中正确处理错误
- 使用回调函数传递错误信息

### 3. 性能考虑

- 避免在同步方法中执行长时间操作
- 使用异步方法处理耗时任务
- 优化数据序列化/反序列化

### 4. 测试

为每个Feature模块编写测试：

```
#[cfg(test)]
mod tests {
    use super::*;
    use mquickjs::Context;

    #[test]
    fn test_sync_method() {
        let impl = MyFeatureImpl;
        let result = impl.do_something("test".to_string()).unwrap();
        assert!(result.contains("Processed"));
    }

    #[tokio::test]
    async fn test_async_method() {
        // 异步测试实现
    }
}
```

## 调试和故障排除

### 1. 生成代码检查

检查ridl-tool生成的代码：

```
# 生成的Rust代码
./target/generated/module_registry.rs

# 生成的C代码
./deps/mquickjs/mqjs_stdlib.c
```

### 2. 常见问题

- **RIDL语法错误**：检查语法是否符合规范
- **类型不匹配**：确保JS和Rust类型正确映射
- **模块未找到**：确认模块已正确注册到映射表

### 3. 日志和调试

在实现中添加适当的日志：

```
use log;

impl MyFeature for MyFeatureImpl {
    fn do_something(&self, input: String) -> Result<String> {
        log::debug!("Processing input: {}", input);
        // 实现逻辑
        Ok(format!("Processed: {}", input))
    }
}
```

## 部署和分发

### 1. 发布准备

- 确保所有功能都经过测试
- 更新版本号
- 编写详细的README文档

### 2. 依赖管理

- 使用适当的版本约束
- 避免不必要的依赖
- 考虑依赖的许可证兼容性

通过遵循本指南，您可以创建功能完整、性能优良的Feature模块，并将其无缝集成到mquickjs系统中。

## 相关文档

- [RIDL_DESIGN.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/RIDL_DESIGN.md) - RIDL设计文档，提供设计原则和语法设计背景
- [RIDL_GRAMMAR_SPEC.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/RIDL_GRAMMAR_SPEC.md) - 词法和文法规范，提供详细语法定义
- [IMPLEMENTATION_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/IMPLEMENTATION_GUIDE.md) - 与 Rust 实现的对应关系和代码生成机制
- [TECH_SELECTION.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/TECH_SELECTION.md) - ridl-tool的技术选型和实现计划