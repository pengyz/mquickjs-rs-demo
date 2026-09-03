---
name: ridl-parser-testing-strategy
description: PEG/pest 解析器测试策略——三层测试、覆盖率分析、错误测试
type: pattern
created: 2026-09-04
sources: [deps/ridl-tool/src/parser/mod.rs, deps/ridl-tool/tests/]
---

## PEG/pest 解析器测试策略

### 三层测试架构

```
Layer 1: 语法单元测试（inline #[test]）
  ↓ 验证每条语法规则独立工作
Layer 2: 集成测试（tests/*.rs）
  ↓ 验证规则组合、AST 转换、边界情况
Layer 3: 端到端测试（ridl → codegen → compile → run）
  ↓ 验证完整管线
```

### Layer 1：语法单元测试（pest 解析验证）

每条语法规则至少 2 个测试：1 个正向 + 1 个反向。

```rust
// 正向：解析成功，验证 AST 结构
#[test]
fn test_class_definition() {
    let result = parse(Rule::class_def, "class Foo { fn bar() -> void; }");
    assert!(result.is_ok());
    // 验证 AST 节点类型、字段名、方法数等
}

// 反向：解析失败，验证错误位置
#[test]
fn test_invalid_class_definition() {
    let result = parse(Rule::class_def, "class { fn bar(); }");
    assert!(result.is_err());
    // 验证错误消息包含位置信息
}
```

**覆盖率检查清单**（60 条语法规则）：

| 类别 | 规则数 | 测试要求 |
|---|---|---|
| 顶层定义 | 9 | 每个正向+反向 |
| 类成员 | 12 | 每个至少 1 个正向 |
| 类型系统 | 11 | 每个正向+反向+嵌套 |
| 参数 | 4 | 正常+可变参数+空 |
| 字面量 | 6 | 每个正向+反向 |
| 标识符 | 2 | 关键字冲突测试 |

### Layer 2：集成测试（AST 结构验证）

验证解析后的 AST 结构正确：

```rust
#[test]
fn test_parse_opaque_block_basic() {
    let input = r#"
        class Node {
            opaque {
                held: Traced<Value>
                count: i32
            }
            fn test() -> void;
        }
    "#;
    let class = parse_class(input).unwrap();
    assert_eq!(class.opaque_fields.len(), 2);
    assert_eq!(class.opaque_fields[0].name, "held");
    assert!(matches!(class.opaque_fields[0].ty, Type::Traced(_)));
    assert_eq!(class.opaque_fields[1].name, "count");
    assert!(matches!(class.opaque_fields[1].ty, Type::Named(_)));
}
```

**关键验证点**：
- 字段数量正确
- 字段名正确
- 类型解析正确（Traced<T>、Option<T>、基本类型）
- 可选分隔符（`;` 和 `,`）都接受
- 嵌套类型正确（`Array<Traced<Value>>`）

### Layer 3：端到端测试

```rust
#[test]
fn test_ridl_to_compiled_code() {
    let ridl = r#"
        class TracedNode {
            opaque { held: Traced<Value> }
            fn finalizerCount() -> i32;
        }
    "#;
    // 1. 解析 RIDL
    let ast = parse_ridl(ridl).unwrap();
    // 2. 生成代码
    let rust_code = generate_rust_api(&ast).unwrap();
    let c_header = generate_c_header(&ast).unwrap();
    // 3. 验证生成的代码包含关键结构
    assert!(rust_code.contains("pub struct TracedNodeOpaque"));
    assert!(rust_code.contains("fn gc_mark"));
    assert!(c_header.contains("gc_mark"));
}
```

### 错误测试策略

**必须测试的错误场景**：

1. **语法错误**：缺少括号、分号、类型
2. **语义错误**：重复字段名、未定义类型引用
3. **边界情况**：空文件、超长标识符、嵌套深度
4. **错误位置**：错误消息必须包含行号和列号

```rust
#[test]
fn test_error_location_reporting() {
    let input = r#"
class Foo {
    fn bar() -> void;
    fn baz( -> void;  // 缺少右括号
}
    "#;
    let err = parse_ridl(input).unwrap_err();
    assert!(err.to_string().contains("line 3"), "错误应指向第3行");
}
```

### 覆盖率分析方法

1. **列出所有语法规则**：`grep "^[a-z_].*=" grammar.pest`
2. **检查每个规则是否有测试**：`grep "fn test_*" mod.rs`
3. **标记未覆盖规则**：创建覆盖率矩阵
4. **优先补充**：类型系统 > 类成员 > 字面量 > 其他

### 当前覆盖率（2026-09-04）

- 语法单元测试：59 个（覆盖 ~70% 规则）
- 集成测试：38 个
- **未覆盖**：proto_var_member, proto_readonly_prop, proto_readwrite_prop, class_constructor, class_constructor_compat, null_literal, mode_decl（仅综合测试）
