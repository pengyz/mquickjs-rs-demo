---
name: ridl-codegen-testing-strategy
description: RIDL 代码生成器测试策略——模板验证、类型映射、gc_mark 生成
type: pattern
created: 2026-09-04
sources: [deps/ridl-tool/src/generator/, deps/ridl-tool/tests/]
---

## RIDL 代码生成器测试策略

### 代码生成器架构

```
RIDl AST → filters.rs (类型映射) → templates/*.j2 (Askama 模板) → 输出文件
                ↓
         mod.rs (模板上下文)
                ↓
         naming.rs (标识符规范化)
```

### 测试层次

#### 1. 类型映射测试（filters.rs）

验证 `rust_type_from_idl` 函数：

```rust
#[test]
fn test_rust_type_from_idl_traced() {
    let ty = Type::Traced(Box::new(Type::Named("Value".into())));
    assert_eq!(rust_type_from_idl(&ty).unwrap(), "mquickjs_rs::Traced<mquickjs_rs::Value>");
}

#[test]
fn test_rust_type_from_idl_optional_traced() {
    let ty = Type::Optional(Box::new(Type::Traced(Box::new(Type::Named("Value".into())))));
    assert_eq!(rust_type_from_idl(&ty).unwrap(), "Option<mquickjs_rs::Traced<mquickjs_rs::Value>>");
}
```

**必须覆盖的类型组合**：
- 基本类型：bool, i32, i64, f32, f64, string, void, object, any
- 复合类型：Array<T>, Map<K,V>, Optional<T>, Union<T|U>
- 特殊类型：Traced<T>, ClassRef, Callback
- 嵌套：Array<Traced<Value>>, Optional<Traced<Value>>, Map<string, Traced<Value>>

#### 2. 模板渲染测试

验证生成的代码结构：

```rust
#[test]
fn test_opaque_struct_generation() {
    let ridl = r#"
        class Node {
            opaque {
                held: Traced<Value>
                count: i32
            }
            fn test() -> void;
        }
    "#;
    let output = generate_rust_api(ridl).unwrap();
    // 验证 opaque struct 定义
    assert!(output.contains("pub struct NodeOpaque"));
    assert!(output.contains("pub held: mquickjs_rs::Traced<mquickjs_rs::Value>"));
    assert!(output.contains("pub count: i32"));
    // 验证 gc_mark 方法
    assert!(output.contains("fn gc_mark(&self, mf: *const"));
    assert!(output.contains("self.held.gc_mark(mf)"));
    // 验证不包含非 Traced 字段的 gc_mark
    assert!(!output.contains("self.count.gc_mark"));
}
```

#### 3. gc_mark 生成测试

验证 gc_mark 函数正确处理各种 Traced<T> 模式：

```rust
#[test]
fn test_gc_mark_with_optional_traced() {
    let ridl = r#"
        class Node {
            opaque { held: Traced<Value>? }
            fn test() -> void;
        }
    "#;
    let output = generate_rust_api(ridl).unwrap();
    // Optional<Traced<T>> 需要 unwrap
    assert!(output.contains("if let Some(ref inner) = self.held"));
    assert!(output.contains("inner.gc_mark(mf)"));
}

#[test]
fn test_gc_mark_without_traced_fields() {
    let ridl = r#"
        class Node {
            opaque { count: i32 }
            fn test() -> void;
        }
    "#;
    let output = generate_rust_api(ridl).unwrap();
    // 无 Traced 字段，不生成 gc_mark
    assert!(!output.contains("fn gc_mark"));
}
```

#### 4. C 头文件生成测试

验证生成的 C 头文件包含正确的声明：

```rust
#[test]
fn test_c_header_gc_mark_declaration() {
    let ridl = r#"
        class Node {
            opaque { held: Traced<Value> }
            fn test() -> void;
        }
    "#;
    let header = generate_c_header(ridl).unwrap();
    // gc_mark 声明
    assert!(header.contains("void js_global_class_node_gc_mark("));
    // JS_CLASS_DEF 包含 gc_mark 参数
    assert!(header.contains("js_global_class_node_gc_mark"));
    // keepalive 引用
    assert!(header.contains("(void)&js_global_class_node_gc_mark"));
}
```

#### 5. 端到端编译测试

验证生成的代码可以编译：

```rust
#[test]
fn test_generated_code_compiles() {
    let ridl = r#"
        class TracedNode {
            opaque { held: Traced<Value> }
            fn finalizerCount() -> i32;
        }
    "#;
    // 生成代码到临时目录
    let tmp = tempfile::tempdir().unwrap();
    generate_to_dir(ridl, tmp.path()).unwrap();
    // 尝试编译
    let output = Command::new("cargo")
        .args(["check", "--manifest-path", &format!("{}/Cargo.toml", tmp.path().display())])
        .output()
        .unwrap();
    assert!(output.status.success(), "生成的代码编译失败: {}", String::from_utf8_lossy(&output.stderr));
}
```

### 测试覆盖率矩阵

| 组件 | 测试文件 | 测试数 | 覆盖率 |
|---|---|---|---|
| 类型映射 | filters.rs (inline) | ~20 | 高 |
| opaque struct | opaque_struct_generation_test.rs | 2 | 中 |
| gc_mark 生成 | gc_mark_generation_test.rs | 4 | 中 |
| C 头文件 | gcmark_render_test.rs | 1 | 低 |
| 错误处理 | error_test.rs | 2 | 低 |
| 端到端 | 无 | 0 | 无 |

### 当前缺口

1. **Traced<T> 类型映射**：只有 1 个测试，需要覆盖嵌套组合
2. **C 头文件渲染**：只有 1 个测试，需要验证 JS_CLASS_DEF 参数顺序
3. **端到端编译**：无测试，需要验证生成代码可编译
4. **错误恢复**：代码生成器的错误消息质量未测试
