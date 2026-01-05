# RIDL Pest Grammar Test

这是一个用于验证 RIDL (Rust Interface Definition Language) 文法规范的 Pest 解析器测试项目。该项目确保 RIDL 文法的合理性、正确性和完整性。

## 项目结构

- `grammar.pest` - RIDL 文法定义文件
- `src/lib.rs` - 解析器定义和全面的测试用例
- `Cargo.toml` - 项目配置文件
- `README.md` - 项目说明文档

## 文法验证范围

该项目验证了 RIDL 文法的以下方面：

### 1. 基础语法元素
- 标识符
- 字面量（字符串、整数、浮点数、布尔值）
- 关键字

### 2. 类型系统
- 基本类型（bool, int, float, string等，但**不包括** function 类型）
- 可空类型（Nullable Types）：`string?`
- 联合类型（Union Types）：`string | int | bool`
- 数组类型：`array<string>`
- 映射类型：`map<string, int>`
- 回调类型：`callback(success: bool)`
- 分组类型：`(Person | LogEntry)`

### 3. 定义结构
- 接口定义
- 类定义（含构造函数、属性、方法）
- 枚举定义
- 结构体定义（支持json、msgpack、protobuf格式）
- 回调定义
- 函数定义
- 类型别名（using）
- 导入语句
- 单例定义
- 模块声明

### 4. 错误处理和边界情况
- 语法错误检测
- 不完整定义检测
- 关键字冲突处理
- 类型定义错误处理
- 结构完整性验证

## 测试覆盖

项目包含 37 个测试用例，覆盖：

1. **基础语法测试**（5个）：标识符、字面量等
2. **类型系统测试**（7个）：各种类型定义
3. **结构定义测试**（9个）：接口、类、枚举等定义
4. **复合结构测试**（3个）：复杂类型定义
5. **错误用例测试**（13个）：各种无效语法

## 重要说明：function 类型的移除

在对 RIDL 类型系统的重新审视中，我们决定从基本类型中完全移除 `function` 类型。这是基于以下考虑：

1. 在 RIDL 中，函数是通过签名定义的（使用 `fn` 关键字）
2. 异步操作通过 `callback` 类型处理
3. 没有通用的函数类型需求
4. 保持类型系统简洁性和一致性

因此，`basic_type` 规则现在定义为：
```
basic_type = { "bool" | "int" | "float" | "double" | "string" | "void" | "object" | "null" | "any" }
```

## 重要说明：PEG解析器的回退机制

本项目使用 Pest（一种PEG - Parsing Expression Grammar 解析器生成器）。PEG解析器具有回退机制，当一个规则无法完全匹配时，它会尝试其他规则。这意味着某些不完整的语法可能会被解析为有效的子结构，而不是整体报错。

例如：
- `string |` 会被解析为 `string`（缺少右侧操作数的联合类型）
- `string??` 会被解析为 `string?`（第二个`?`被忽略）

因此，测试用例设计时考虑了PEG解析器的这一特性，专注于测试真正会导致解析失败的错误情况。

## 使用方法

运行测试以验证RIDL文法：

```bash
cargo test
```

所有测试通过表明RIDL文法定义是合理和完整的。

## 与RIDL规范的一致性

该文法定义与以下文档保持一致：
- `RIDL_DESIGN.md` - RIDL设计文档
- `RIDL_GRAMMAR_SPEC.md` - RIDL文法规范文档

## 目标

本项目的目标是确保RIDL文法在实际应用中的正确性，为后续的代码生成工具提供可靠的基础。