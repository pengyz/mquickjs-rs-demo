# ridl-tool 技术选型文档

## 概述

本文档详细说明了 ridl-tool（mquickjs IDL 代码生成工具）的技术选型和实现计划。

**注意：当前文档描述的是ridl-tool的设计方案和实现计划。由于RIDL工具链（ridl-tool）尚未完成，目前的实现是通过手动编写胶水代码和实现代码来验证整个流程。一旦ridl-tool完成，将按照本文档选择的技术方案进行实现。**

## 1. 语法解析器选型

### 1.1 选项对比

| 解析器工具 | 优点 | 缺点 | 推荐度 |
|------------|------|------|--------|
| **nom** | 零成本抽象，性能好，完全在 Rust 中实现 | 学习曲线陡峭，错误处理复杂 | 高 |
| **pest** | 语法简洁，错误处理好，性能优秀 | 需要额外的语法定义文件 | 高 |
| **combine** | 灵活，组合子解析 | 性能稍差，API 复杂 | 中 |
| **lalrpop** | 生成 LALR 解析器，性能好 | 语法复杂，学习曲线陡峭 | 中 |

### 1.2 推荐方案：pest

选择 **pest** 作为解析器生成工具，原因如下：

1. **语法简洁**：使用 PEG 语法定义，易于理解和维护
2. **错误处理优秀**：提供良好的错误定位和提示
3. **性能良好**：在性能和易用性之间取得良好平衡
4. **活跃维护**：社区活跃，文档完善

## 2. 代码生成引擎选型

### 2.1 选项对比

| 模板引擎 | 优点 | 缺点 | 推荐度 |
|----------|------|------|--------|
| **Tera** | Django/Jinja2 风格，功能强大，支持继承和宏 | 重量级，可能过于复杂 | 中 |
| **Handlebars** | 逻辑无关，类型安全，性能好 | 逻辑处理能力有限 | 高 |
| **Askama** | 编译时模板，类型安全，性能极好 | 编译时确定模板，灵活性稍差 | 高 |
| **Liquid** | Shopify 开源，安全沙箱 | 性能一般 | 中 |

### 2.2 推荐方案：Askama

选择 **Askama** 作为模板引擎，原因如下：

1. **编译时处理**：模板在编译时展开，运行时性能极好
2. **类型安全**：Rust 类型系统保证模板变量类型安全
3. **性能优异**：零运行时开销，生成的代码与手写代码性能相当
4. **简单易用**：语法简洁，易于学习和使用

## 3. 项目结构设计

```
ridl-tool/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── parser/           # 解析器模块
│   │   ├── mod.rs
│   │   ├── grammar.rs    # pest 语法定义
│   │   └── ast.rs        # 抽象语法树定义
│   ├── generator/        # 代码生成模块
│   │   ├── mod.rs
│   │   ├── rust_glue.rs  # Rust 胶水代码生成
│   │   ├── c_binding.rs  # C 绑定代码生成
│   │   └── templates/    # 模板文件
│   └── cli.rs           # 命令行接口
├── doc/                 # 文档目录
│   ├── IDL_DESIGN.md
│   └── IMPLEMENTATION_GUIDE.md
└── tests/               # 测试目录
    ├── integration/
    └── fixtures/
        ├── valid/
        └── invalid/
```

## 4. 实现计划

### 4.1 第一阶段：基础解析器
- 定义 pest 语法文件
- 实现 AST 结构
- 解析基本类型定义

### 4.2 第二阶段：完整解析
- 支持所有 IDL 语法元素
- 实现错误处理和验证
- 添加单元测试

### 4.3 第三阶段：代码生成
- 使用 Askama 实现 Rust 胶水代码生成
- 生成 C 绑定代码
- 生成标准库描述代码

### 4.4 第四阶段：工具整合
- 命令行接口
- 集成测试
- 文档完善

## 5. 依赖配置

```toml
[dependencies]
pest = "2.0"
pest_derive = "2.0"
askama = "0.12"
clap = { version = "4.0", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 6. 关键实现细节

### 6.1 语法定义 (grammar.rs)
```pest
// IDL 语法定义
idl = { SOI ~ (definition)* ~ EOI }
definition = { interface_def | class_def | struct_def | enum_def | function_def }
interface_def = { "interface" ~ identifier ~ "{" ~ (method_def)* ~ "}" }
// ... 更多语法规则
```

### 6.2 AST 结构 (ast.rs)
```rust
#[derive(Debug)]
pub enum Definition {
    Interface(Interface),
    Class(Class),
    Struct(Struct),
    Enum(Enum),
    Function(Function),
}

#[derive(Debug)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
}
// ... 更多 AST 结构
```

### 6.3 代码生成 (generator/)
使用 Askama 模板生成 Rust 和 C 代码：

```jinja2
{# Rust 胶水代码模板 #}
impl {{ interface.name }} for {{ implementation }} {
{% for method in interface.methods %}
    fn {{ method.name }}(&self, {% for param in method.params %}{{ param.name }}: {{ param.type }}{% if !loop.last %}, {% endif %}{% endfor %}) -> Result<{{ method.return_type }}, String> {
        // 实现逻辑
    }
{% endfor %}
}
```

## 7. 测试策略

- 单元测试：验证解析器和生成器功能
- 集成测试：端到端测试，验证生成的代码可以正确编译
- 快照测试：确保生成的代码格式和内容正确

## 8. 错误处理

- 语法错误：提供清晰的错误位置和描述
- 语义错误：验证类型定义和引用的正确性
- 生成错误：处理模板渲染和代码生成过程中的错误

这个技术选型方案将为我们提供一个高性能、类型安全且易于维护的 IDL 代码生成工具。

## 相关文档

- [RIDL_DESIGN.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/RIDL_DESIGN.md) - RIDL设计文档，提供设计原则和语法设计背景
- [RIDL_GRAMMAR_SPEC.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/RIDL_GRAMMAR_SPEC.md) - 词法和文法规范，提供详细语法定义
- [IMPLEMENTATION_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/IMPLEMENTATION_GUIDE.md) - 与 Rust 实现的对应关系和代码生成机制
- [FEATURE_DEVELOPMENT_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/FEATURE_DEVELOPMENT_GUIDE.md) - 如何开发和集成基于RIDL的Feature模块