# RIDL工具多文件聚合生成设计文档

## 概述

本文档详细描述了RIDL工具如何处理多个RIDL文件并将其定义聚合到共享文件中的设计和实现方案。该方案通过Askama模板系统和中间数据收集机制，确保所有模块定义正确合并到共享的头文件和符号文件中。

## 设计目标

1. **多文件支持**：支持同时处理多个RIDL文件
2. **模板化生成**：使用Askama模板系统替代字符串拼接
3. **内容聚合**：将所有模块定义正确合并到共享文件
4. **向后兼容**：保持对单文件处理的支持

## 核心设计原则

### 1. 数据收集与批量处理
- 解析所有RIDL文件后，先收集定义到中间数据结构
- 再统一使用模板生成共享文件
- 避免逐个文件追加导致的覆盖问题

### 2. 模块与共享文件分离
- 模块专属文件（`{module}_glue.rs`、`{module}_impl.rs`）：基于单个RIDL文件生成
- 共享文件（`mquickjs_ridl_register.h`、`ridl_symbols.rs`）：聚合所有模块定义

### 3. Askama模板系统
- 完全使用Askama模板系统生成所有文件内容
- 禁止字符串拼接方式生成代码
- 提高代码可维护性和一致性

## 数据结构设计

### 1. 中间数据收集结构

```rust
#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    pub module_name: String,      // 从RIDL文件名提取的模块名
    pub interfaces: Vec<Interface>, // 该模块的接口定义
}

#[derive(Debug, Clone)]
pub struct AllDefinitions {
    pub modules: Vec<ModuleDefinition>, // 所有模块的定义集合
}
```

### 2. 模板结构

```rust
#[derive(Template)]
#[template(path = "c_header.rs.j2", escape = "none")]
pub struct CHeaderTemplate<'a> {
    pub all_definitions: &'a AllDefinitions,
}

#[derive(Template)]
#[template(path = "symbols.rs.j2", escape = "none")]
pub struct SymbolsTemplate<'a> {
    pub all_definitions: &'a AllDefinitions,
}
```

## 分阶段生成方案（最新）

### 背景
在实际项目架构中，RIDL模块作为独立的Rust crate存在，每个模块独立编译。依赖关系为：
- RIDL模块依赖mquickjs-rs
- mquickjs-rs依赖mquickjs.a（只含核心库，不含标准库）
- 标准库编译放在mquickjs-demo中

### 方案设计
将RIDL工具的生成过程分为两个阶段：

#### 阶段1：模块级文件生成（在各RIDL模块的构建过程中）
- 每个RIDL模块在自己的build.rs中调用`ridl-tool`生成`{module}_glue.rs`和`{module}_impl.rs`
- 生成的文件放置在模块的输出目录中
- 命令：`ridl-tool module <ridl_file> [output_dir]`

#### 阶段2：全局聚合文件生成（在mquickjs-demo的构建过程中）
- 在mquickjs-demo的build.rs中收集所有RIDL模块的RIDL文件
- 调用`ridl-tool`生成统一的mquickjs_ridl_register.h和ridl_symbols.rs
- 命令：`ridl-tool aggregate <ridl_file1> <ridl_file2> ... [output_dir]`

### 实施细节

#### 1. 模块级文件生成
- 每个RIDL模块独立处理
- 生成模块专属的Rust胶水代码和实现代码
- 不影响其他模块的构建过程

#### 2. 全局聚合文件生成
- 在最终项目构建时统一处理
- 收集所有RIDL文件并生成共享文件
- 确保所有模块都被正确聚合

### 优势
- 保持了模块的独立性
- 每个模块可以在自己的构建过程中生成所需的代码
- 最终聚合在主项目中完成，确保所有模块都被包含
- 避免了一次性传递所有RIDL文件的问题

## 实现流程

### 模块级生成流程
```
输入: [file1.ridl] (单个文件)
↓
解析RIDL文件 → AST
↓
验证AST
↓
提取Interface定义
↓
生成模块专属文件:
  - file1_glue.rs (使用RustGlueTemplate)
  - file1_impl.rs (使用RustImplTemplate)
```

### 全局聚合生成流程
```
输入: [file1.ridl, file2.ridl, ...] (多个文件)
↓
解析每个RIDL文件 → AST
↓
验证每个AST
↓
遍历所有AST，提取Interface定义
↓
构建ModuleDefinition
↓
收集到AllDefinitions结构
↓
生成共享文件:
  - mquickjs_ridl_register.h (使用CHeaderTemplate)
  - ridl_symbols.rs (使用SymbolsTemplate)
```

## 模板设计

### 1. C头文件模板 (c_header.rs.j2)

```jinja2
/*
 * Generated header file for RIDL-defined standard library extensions
 */

#ifndef MQUICKJS_RIDL_REGISTER_H
#define MQUICKJS_RIDL_REGISTER_H

#include "mquickjs.h"

// Function declarations for all RIDL-defined functions
{%- for module in all_definitions.modules -%}
{%- for interface in module.interfaces -%}
{%- for method in interface.methods -%}
JSValue js_{{ interface.name|lower }}_{{ method.name }}(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);
{%- endfor -%}
{%- endfor -%}
{%- endfor -%}

{%- for module in all_definitions.modules -%}
// Define RIDL extensions for module {{ module.module_name }}
#define JS_STDLIB_EXTENSIONS_{{ module.module_name|upper }} \
{%- for interface in module.interfaces -%}
{%- for method in interface.methods -%}
    JS_CFUNC_DEF("{{ method.name }}", {{ method.params.len() }}, js_{{ interface.name|lower }}_{{ method.name }}), \
{%- endfor -%}
{%- endfor %}

{%- endfor -%}

#define JS_RIDL_EXTENSIONS \
    JS_STDLIB_EXTENSIONS \
{%- for module in all_definitions.modules -%}
    JS_STDLIB_EXTENSIONS_{{ module.module_name|upper }} \
{%- endfor %}

#endif /* MQUICKJS_RIDL_REGISTER_H */
```

### 2. 符号文件模板 (symbols.rs.j2)

```jinja2
// Generated symbol references for RIDL extensions
{%- for module in all_definitions.modules -%}
{%- for interface in module.interfaces -%}
{%- for method in interface.methods -%}
use crate::{{ module.module_name }}_glue::{{ interface.name|lower }}_{{ method.name }}_glue;
{%- endfor -%}
{%- endfor -%}
{%- endfor %}

// Use all glue functions to ensure they're linked
pub fn ensure_symbols() {
{%- for module in all_definitions.modules -%}
{%- for interface in module.interfaces -%}
{%- for method in interface.methods -%}
    let _ = {{ interface.name|lower }}_{{ method.name }}_glue;
{%- endfor -%}
{%- endfor -%}
{%- endfor %}
}
```

## API设计

### 1. 命令行接口

```bash
# 模块级生成
ridl-tool module <ridl_file> [output_dir]

# 全局聚合生成
ridl-tool aggregate <ridl_file1> <ridl_file2> ... [output_dir]
```

### 2. 辅助函数

```rust
// 生成模块特定文件
pub fn generate_module_files(
    items: &[IDLItem], 
    output_dir: &str, 
    module_name: &str
) -> Result<(), Box<dyn std::error::Error>>;

// 生成共享文件
pub fn generate_shared_files(
    ridl_files: &[String], 
    output_dir: &str
) -> Result<(), Box<dyn std::error::Error>>;
```

## 错误处理

### 1. 解析错误
- 任何一个RIDL文件解析失败，整个过程终止
- 提供详细的错误位置和原因

### 2. 验证错误
- 验证所有文件后才开始生成
- 避免部分生成导致的不一致状态

### 3. 文件I/O错误
- 确保输出目录存在
- 生成过程中任何写入失败立即终止

## 向后兼容性

### 1. 单文件支持
保留原有的单文件处理能力，通过统一接口实现：

```rust
// 兼容原有接口
pub fn generate_code(
    items: &[IDLItem], 
    output_dir: &str, 
    module_name: Option<&str>
) -> Result<(), Box<dyn std::error::Error>> {
    // 实现单文件处理逻辑
}
```

### 2. 命令行接口
支持多种使用方式：
- 单文件处理：`ridl-tool file.ridl [output_dir]`
- 多文件处理：`ridl-tool file1.ridl file2.ridl ... [output_dir]`

## 测试策略

### 1. 单元测试
- 测试数据结构的正确性
- 测试模板渲染结果
- 测试错误处理逻辑

### 2. 集成测试
- 测试多文件聚合生成
- 验证生成文件的语法正确性
- 测试与mquickjs的集成

## 性能考虑

### 1. 内存使用
- 避免重复存储解析结果
- 使用引用而非复制来减少内存占用

### 2. 生成效率
- 批量处理减少I/O操作次数
- 模板预编译提升渲染速度

## 维护指南

### 1. 模板修改
- 修改模板后需要测试所有相关功能
- 验证生成代码的语法正确性

### 2. 接口变更
- 保持向后兼容性
- 提供迁移指南

## 已知限制

1. 所有RIDL文件必须在一次运行中提供，不支持增量添加
2. 模块名冲突需要用户自行解决
3. 模板语法错误可能在运行时才发现

## 未来扩展

1. 支持增量生成
2. 提供更丰富的模板定制选项
3. 集成到构建系统中自动执行