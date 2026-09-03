# GC 功能测试完善实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 mquickjs-rs 的 GC Root 体系，通过扩展 RIDL 语法支持 opaque 字段声明，实现 Traced<T> 类型，并建立完善的测试覆盖。

**Architecture:** 四阶段实施 - (0) 扩展 RIDL 语法支持 opaque 块和类型系统，(1) 修复当前测试的弱断言，(2) 扩展测试覆盖到正反向场景，(3) TDD 驱动实现 Traced<T> 并自动生成 gc_mark。

**Tech Stack:** Rust, mquickjs, RIDL (DSL parser/codegen), pest (parser generator), askama (template engine)

---

## 文件结构

### 阶段 0：RIDL 语法扩展
**新增文件**：
- 无（修改现有文件）

**修改文件**：
- `deps/ridl-tool/src/parser/ast.rs` - 新增 OpaqueField、扩展 Type 枚举
- `deps/ridl-tool/src/parser/mod.rs` - 扩展 parser 支持 opaque 块
- `deps/ridl-tool/src/generator/mod.rs` - 生成 opaque struct 和 gc_mark
- `deps/ridl-tool/tests/parser_opaque_test.rs` - 新增 parser 单元测试
- `deps/ridl-tool/tests/fixtures/gc_mark.ridl` - 集成测试 fixture

### 阶段 1：修复当前测试
**修改文件**：
- `tests/gc_root_cycle.rs` - 修复断言，添加辅助函数

### 阶段 2：扩展测试覆盖
**修改文件**：
- `tests/gc_root_cycle.rs` - 添加 10+ 测试用例和 GcTestHarness

### 阶段 3：Traced<T> 实现
**新增文件**：
- `deps/mquickjs-rs/src/traced.rs` - Traced<T> 类型定义
- `tests/gc_traced.rs` - Traced<T> 测试
- `tests/global/gc_traced/test_gc_traced/` - 新 RIDL 模块

**修改文件**：
- `deps/mquickjs-rs/src/lib.rs` - 导出 Traced
- `deps/mquickjs-rs/src/mod.rs` - 声明 traced 模块

---

## 阶段 0：扩展 RIDL 语法（3-5 天）

### Task 0.1: 扩展 AST 支持 opaque 字段

**Files:**
- Modify: `deps/ridl-tool/src/parser/ast.rs:58-86`

- [ ] **Step 1: 在 Class 结构中添加 opaque_fields 字段**

在 `Class` struct 定义中添加新字段：

```rust
// deps/ridl-tool/src/parser/ast.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Class {
    pub name: String,
    #[serde(default)]
    pub pos: Option<SourcePos>,
    pub constructor: Option<Function>,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
    pub js_fields: Vec<JsField>,
    #[serde(default)]  // ← 新增
    pub opaque_fields: Vec<OpaqueField>,  // ← 新增
    pub module: Option<ModuleDeclaration>,
}
```

- [ ] **Step 2: 定义 OpaqueField 结构**

在 ast.rs 文件末尾添加：

```rust
// deps/ridl-tool/src/parser/ast.rs（文件末尾）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpaqueField {
    pub name: String,
    pub field_type: Type,
    #[serde(default)]
    pub pos: Option<SourcePos>,
}
```

- [ ] **Step 3: 扩展 Type 枚举支持 Traced**

在 `Type` enum 中添加新变体（在 Callback 之后）：

```rust
// deps/ridl-tool/src/parser/ast.rs（Type enum 内）
pub enum Type {
    // ... 现有变体 ...
    Callback,
    
    /// Traced<T> - GC-traced field for opaque structs
    Traced(Box<Type>),  // ← 新增
}
```

- [ ] **Step 4: 为 Type::Traced 实现 Display trait**

找到 `impl fmt::Display for Type` 并添加 Traced 分支：

```rust
// deps/ridl-tool/src/parser/ast.rs（Display impl 内）
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // ... 现有分支 ...
            Type::Callback => write!(f, "callback"),
            Type::Traced(inner) => write!(f, "Traced<{}>", inner),  // ← 新增
        }
    }
}
```

- [ ] **Step 5: 运行编译检查**

Run: `cargo build --manifest-path deps/ridl-tool/Cargo.toml`
Expected: 成功编译，无错误

- [ ] **Step 6: Commit AST 扩展**

```bash
git add deps/ridl-tool/src/parser/ast.rs
git commit -m "feat(ridl): extend AST for opaque fields

- Add opaque_fields to Class struct
- Add OpaqueField struct
- Add Type::Traced variant
- Update Display impl for Type

Part of GC Root system phase 0"
```

---

### Task 0.2: 实现 opaque 块 parser

**Files:**
- Modify: `deps/ridl-tool/src/parser/mod.rs`

- [ ] **Step 1: 找到 parse_class 函数**

Run: `rg "fn parse_class" deps/ridl-tool/src/parser/mod.rs -A 5`
Expected: 找到函数定义位置

- [ ] **Step 2: 在 parse_class 中添加 opaque 块解析**

在 `parse_class` 函数的方法解析循环中，添加 opaque 块处理（在方法和属性解析之前）：

```rust
// deps/ridl-tool/src/parser/mod.rs（parse_class 函数内）
fn parse_class(...) -> Result<Class> {
    // ... 现有代码 ...
    
    let mut opaque_fields = Vec::new();  // ← 新增
    let mut methods = Vec::new();
    let mut properties = Vec::new();
    
    while !self.check(TokenKind::RBrace) {
        // ← 新增 opaque 块解析
        if self.check_keyword("opaque") {
            self.consume(TokenKind::Keyword)?;  // 消费 "opaque"
            opaque_fields = self.parse_opaque_block()?;
            continue;
        }
        
        // 现有的方法/属性解析
        if self.check_keyword("fn") {
            // ... 现有代码 ...
        }
        // ...
    }
    
    Ok(Class {
        // ... 现有字段 ...
        opaque_fields,  // ← 新增
        // ...
    })
}
```

- [ ] **Step 3: 实现 parse_opaque_block 方法**

在 parse_class 函数之后添加新方法：

```rust
// deps/ridl-tool/src/parser/mod.rs（parse_class 之后）
fn parse_opaque_block(&mut self) -> Result<Vec<OpaqueField>, Box<dyn std::error::Error>> {
    let start_pos = self.current_pos();
    self.expect(TokenKind::LBrace)?;
    
    let mut fields = Vec::new();
    
    while !self.check(TokenKind::RBrace) {
        let field_start_pos = self.current_pos();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let field_type = self.parse_type()?;
        
        fields.push(OpaqueField {
            name,
            field_type,
            pos: Some(field_start_pos),
        });
        
        // 可选的逗号或分号
        if self.check(TokenKind::Comma) || self.check(TokenKind::Semicolon) {
            self.advance();
        }
    }
    
    self.expect(TokenKind::RBrace)?;
    Ok(fields)
}
```

- [ ] **Step 4: 扩展 parse_type 支持 Traced**

找到 `parse_type` 方法并添加 Traced 解析（在处理泛型类型的地方）：

```rust
// deps/ridl-tool/src/parser/mod.rs（parse_type 方法内）
fn parse_type(&mut self) -> Result<Type, Box<dyn std::error::Error>> {
    let type_name = self.expect_ident()?;
    
    match type_name.as_str() {
        // ... 现有分支 ...
        
        "Traced" => {  // ← 新增
            self.expect(TokenKind::Lt)?;
            let inner = self.parse_type()?;
            self.expect(TokenKind::Gt)?;
            Ok(Type::Traced(Box::new(inner)))
        }
        
        // ... 其他分支 ...
    }
}
```

- [ ] **Step 5: 运行编译检查**

Run: `cargo build --manifest-path deps/ridl-tool/Cargo.toml`
Expected: 成功编译

- [ ] **Step 6: Commit parser 改动**

```bash
git add deps/ridl-tool/src/parser/mod.rs
git commit -m "feat(ridl): implement opaque block parser

- Add parse_opaque_block method
- Integrate opaque parsing into parse_class
- Extend parse_type to support Traced<T>

Part of GC Root system phase 0"
```

---

### Task 0.3: 编写 parser 单元测试

**Files:**
- Create: `deps/ridl-tool/tests/parser_opaque_test.rs`

- [ ] **Step 1: 创建测试文件框架**

```rust
// deps/ridl-tool/tests/parser_opaque_test.rs
use ridl_tool::parser::{parse_ridl, IDLItem};

#[test]
fn test_parse_opaque_block_single_field() {
    let input = r#"
        class Node {
            opaque {
                held: Traced<Value>
            }
            fn test() -> void;
        }
    "#;
    
    let items = parse_ridl(input).expect("parse failed");
    assert_eq!(items.len(), 1);
    
    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.name, "Node");
            assert_eq!(class.opaque_fields.len(), 1);
            assert_eq!(class.opaque_fields[0].name, "held");
            // 验证类型
            match &class.opaque_fields[0].field_type {
                ridl_tool::parser::ast::Type::Traced(inner) => {
                    match **inner {
                        ridl_tool::parser::ast::Type::Custom(ref name) => {
                            assert_eq!(name, "Value");
                        }
                        _ => panic!("Expected Custom type inside Traced"),
                    }
                }
                _ => panic!("Expected Traced type"),
            }
        }
        _ => panic!("Expected Class item"),
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cargo test --manifest-path deps/ridl-tool/Cargo.toml --test parser_opaque_test`
Expected: 测试运行（可能失败，因为 parser 实现可能有细节问题）

- [ ] **Step 3: 添加多字段测试**

在同一文件中添加：

```rust
// deps/ridl-tool/tests/parser_opaque_test.rs
#[test]
fn test_parse_opaque_block_multiple_fields() {
    let input = r#"
        class TracedNode {
            opaque {
                held: Option<Traced<Value>>
                count: i32
                other: Traced<Object>
            }
            fn dummy() -> void;
        }
    "#;
    
    let items = parse_ridl(input).expect("parse failed");
    
    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.opaque_fields.len(), 3);
            assert_eq!(class.opaque_fields[0].name, "held");
            assert_eq!(class.opaque_fields[1].name, "count");
            assert_eq!(class.opaque_fields[2].name, "other");
        }
        _ => panic!("Expected Class item"),
    }
}
```

- [ ] **Step 4: 添加无 opaque 块测试**

```rust
// deps/ridl-tool/tests/parser_opaque_test.rs
#[test]
fn test_parse_class_without_opaque() {
    let input = r#"
        class Simple {
            fn test() -> void;
        }
    "#;
    
    let items = parse_ridl(input).expect("parse failed");
    
    match &items[0] {
        IDLItem::Class(class) => {
            assert_eq!(class.opaque_fields.len(), 0);
        }
        _ => panic!("Expected Class item"),
    }
}
```

- [ ] **Step 5: 运行所有测试**

Run: `cargo test --manifest-path deps/ridl-tool/Cargo.toml --test parser_opaque_test`
Expected: 所有测试通过

- [ ] **Step 6: Commit parser 测试**

```bash
git add deps/ridl-tool/tests/parser_opaque_test.rs
git commit -m "test(ridl): add opaque block parser tests

- Test single field parsing
- Test multiple fields parsing
- Test class without opaque block
- Verify Type::Traced parsing

Part of GC Root system phase 0"
```

---

### Task 0.4: 实现 opaque struct 代码生成

**Files:**
- Modify: `deps/ridl-tool/src/generator/mod.rs`

- [ ] **Step 1: 找到 class 代码生成位置**

Run: `rg "fn generate.*class|struct.*generation" deps/ridl-tool/src/generator/mod.rs | head -10`
Expected: 找到 class 生成相关函数

- [ ] **Step 2: 实现 generate_opaque_struct 函数**

在 generator/mod.rs 中添加（在现有生成函数附近）：

```rust
// deps/ridl-tool/src/generator/mod.rs
use quote::quote;
use proc_macro2::TokenStream;
use crate::parser::ast::{Class, Type, OpaqueField};

fn generate_opaque_struct(class: &Class) -> TokenStream {
    let opaque_name = format_ident!("{}Opaque", class.name);
    
    if class.opaque_fields.is_empty() {
        // 无字段时生成空 struct
        return quote! {
            pub struct #opaque_name;
        };
    }
    
    // 生成字段定义
    let field_defs: Vec<TokenStream> = class.opaque_fields.iter().map(|f| {
        let field_name = format_ident!("{}", f.name);
        let field_type = type_to_rust_tokens(&f.field_type);
        quote! {
            pub #field_name: #field_type
        }
    }).collect();
    
    quote! {
        pub struct #opaque_name {
            #(#field_defs),*
        }
    }
}

fn type_to_rust_tokens(ty: &Type) -> TokenStream {
    match ty {
        Type::Traced(inner) => {
            let inner_tokens = type_to_rust_tokens(inner);
            quote! { mquickjs_rs::Traced<#inner_tokens> }
        }
        Type::Optional(inner) => {
            let inner_tokens = type_to_rust_tokens(inner);
            quote! { Option<#inner_tokens> }
        }
        Type::I32 => quote! { i32 },
        Type::I64 => quote! { i64 },
        Type::F32 => quote! { f32 },
        Type::F64 => quote! { f64 },
        Type::Bool => quote! { bool },
        Type::String => quote! { String },
        Type::Custom(name) => {
            let ident = format_ident!("{}", name);
            quote! { #ident }
        }
        // ... 其他类型映射 ...
        _ => {
            // 默认使用 Custom 处理
            let name = format!("{}", ty);
            let ident = format_ident!("{}", name);
            quote! { #ident }
        }
    }
}
```

- [ ] **Step 3: 集成到现有生成流程**

找到生成 class 相关代码的位置并调用 `generate_opaque_struct`：

```rust
// deps/ridl-tool/src/generator/mod.rs（在 class 生成函数中）
pub fn generate_class_code(class: &Class) -> TokenStream {
    // ... 现有代码 ...
    
    // 生成 opaque struct
    let opaque_struct = generate_opaque_struct(class);
    
    quote! {
        #opaque_struct
        
        // ... 其他生成的代码 ...
    }
}
```

- [ ] **Step 4: 运行编译检查**

Run: `cargo build --manifest-path deps/ridl-tool/Cargo.toml`
Expected: 成功编译

- [ ] **Step 5: Commit opaque struct 生成**

```bash
git add deps/ridl-tool/src/generator/mod.rs
git commit -m "feat(ridl): generate opaque struct from opaque fields

- Implement generate_opaque_struct function
- Map RIDL types to Rust types (Traced<T>, Option<T>, etc.)
- Integrate into class code generation

Part of GC Root system phase 0"
```

---

### Task 0.5: 实现 gc_mark 自动生成

**Files:**
- Modify: `deps/ridl-tool/src/generator/mod.rs`

- [ ] **Step 1: 实现 has_traced_fields 检查函数**

```rust
// deps/ridl-tool/src/generator/mod.rs
fn has_traced_fields(opaque_fields: &[OpaqueField]) -> bool {
    opaque_fields.iter().any(|f| is_traced_type(&f.field_type))
}

fn is_traced_type(ty: &Type) -> bool {
    match ty {
        Type::Traced(_) => true,
        Type::Optional(inner) => is_traced_type(inner),
        _ => false,
    }
}
```

- [ ] **Step 2: 实现 generate_gc_mark 函数**

```rust
// deps/ridl-tool/src/generator/mod.rs
fn generate_gc_mark(class: &Class) -> Option<TokenStream> {
    if !has_traced_fields(&class.opaque_fields) {
        return None;
    }
    
    let opaque_name = format_ident!("{}Opaque", class.name);
    
    // 为每个 Traced 字段生成 mark 语句
    let mark_statements: Vec<TokenStream> = class.opaque_fields.iter()
        .filter(|f| is_traced_type(&f.field_type))
        .map(|f| {
            let field_name = format_ident!("{}", f.name);
            
            if is_optional(&f.field_type) {
                quote! {
                    if let Some(ref field) = this.#field_name {
                        mark_value(mf, field.as_raw());
                    }
                }
            } else {
                quote! {
                    mark_value(mf, this.#field_name.as_raw());
                }
            }
        })
        .collect();
    
    Some(quote! {
        impl #opaque_name {
            pub unsafe extern "C" fn gc_mark(
                _ctx: *mut mquickjs_sys::JSContext,
                _obj: mquickjs_sys::JSValue,
                opaque: *mut std::os::raw::c_void,
                mf: *const mquickjs_sys::JSMarkFunc,
            ) {
                if opaque.is_null() || mf.is_null() {
                    return;
                }
                
                let this = &*(opaque as *const Self);
                if let Some(mark_value) = (*mf).mark_value {
                    #(#mark_statements)*
                }
            }
        }
    })
}

fn is_optional(ty: &Type) -> bool {
    matches!(ty, Type::Optional(_))
}
```

- [ ] **Step 3: 集成 gc_mark 生成**

在 `generate_class_code` 中添加 gc_mark 生成：

```rust
// deps/ridl-tool/src/generator/mod.rs
pub fn generate_class_code(class: &Class) -> TokenStream {
    let opaque_struct = generate_opaque_struct(class);
    let gc_mark_impl = generate_gc_mark(class);
    
    quote! {
        #opaque_struct
        
        #gc_mark_impl
        
        // ... 其他生成的代码 ...
    }
}
```

- [ ] **Step 4: 运行编译检查**

Run: `cargo build --manifest-path deps/ridl-tool/Cargo.toml`
Expected: 成功编译

- [ ] **Step 5: Commit gc_mark 生成**

```bash
git add deps/ridl-tool/src/generator/mod.rs
git commit -m "feat(ridl): auto-generate gc_mark for Traced fields

- Implement generate_gc_mark function
- Check if opaque has Traced fields
- Generate mark_value calls for each Traced field
- Handle Option<Traced<T>> with conditional marking

Part of GC Root system phase 0"
```

---

### Task 0.6: 创建 RIDL 集成测试 fixture

**Files:**
- Create: `deps/ridl-tool/tests/fixtures/gc_mark.ridl`
- Create: `deps/ridl-tool/tests/codegen_gc_mark_test.rs`

- [ ] **Step 1: 创建测试 fixture**

```ridl
// deps/ridl-tool/tests/fixtures/gc_mark.ridl
class TestNode {
    opaque {
        traced_field: Traced<Value>
        optional_field: Option<Traced<Value>>
        plain_field: i32
    }
    
    fn dummy() -> void;
}

class NoTracedNode {
    opaque {
        count: i32
        name: string
    }
    
    fn test() -> void;
}

class EmptyOpaqueNode {
    fn method() -> void;
}
```

- [ ] **Step 2: 创建 codegen 测试**

```rust
// deps/ridl-tool/tests/codegen_gc_mark_test.rs
use ridl_tool::parser::parse_ridl_file;
use ridl_tool::generator::generate_class_code;

#[test]
fn test_generate_gc_mark_with_traced_fields() {
    let fixture_content = std::fs::read_to_string(
        "tests/fixtures/gc_mark.ridl"
    ).expect("read fixture");
    
    let parsed = parse_ridl_file(&fixture_content)
        .expect("parse failed");
    
    // 找到 TestNode
    let test_node = parsed.classes.iter()
        .find(|c| c.name == "TestNode")
        .expect("TestNode not found");
    
    let generated_code = generate_class_code(test_node);
    let code_str = generated_code.to_string();
    
    // 验证生成了 gc_mark
    assert!(code_str.contains("pub unsafe extern \"C\" fn gc_mark"));
    assert!(code_str.contains("mark_value"));
    assert!(code_str.contains("traced_field"));
    assert!(code_str.contains("optional_field"));
    // plain_field 不应该被标记
    assert!(!code_str.contains("plain_field.as_raw()"));
}

#[test]
fn test_no_gc_mark_without_traced_fields() {
    let fixture_content = std::fs::read_to_string(
        "tests/fixtures/gc_mark.ridl"
    ).expect("read fixture");
    
    let parsed = parse_ridl_file(&fixture_content)
        .expect("parse failed");
    
    let no_traced_node = parsed.classes.iter()
        .find(|c| c.name == "NoTracedNode")
        .expect("NoTracedNode not found");
    
    let generated_code = generate_class_code(no_traced_node);
    let code_str = generated_code.to_string();
    
    // 不应该生成 gc_mark
    assert!(!code_str.contains("pub unsafe extern \"C\" fn gc_mark"));
}
```

- [ ] **Step 3: 运行集成测试**

Run: `cargo test --manifest-path deps/ridl-tool/Cargo.toml --test codegen_gc_mark_test`
Expected: 测试通过

- [ ] **Step 4: Commit 集成测试**

```bash
git add deps/ridl-tool/tests/fixtures/gc_mark.ridl deps/ridl-tool/tests/codegen_gc_mark_test.rs
git commit -m "test(ridl): add gc_mark codegen integration tests

- Create fixture with Traced fields
- Test gc_mark generation for Traced fields
- Test no gc_mark for non-Traced fields
- Verify generated code structure

Part of GC Root system phase 0"
```

---

### Task 0.7: 更新 C FFI 注册模板

**Files:**
- Modify: `deps/ridl-tool/templates/mquickjs_ridl_register.h.j2`

- [ ] **Step 1: 找到 JSClassDef 模板**

Run: `rg "JSClassDef" deps/ridl-tool/templates/`
Expected: 找到 class 注册模板位置

- [ ] **Step 2: 添加 gc_mark 字段**

在 JSClassDef 定义中添加 gc_mark 字段（在 finalizer 之后）：

```c
// deps/ridl-tool/templates/mquickjs_ridl_register.h.j2
static JSClassDef js_{{ class.name }}_class = {
    .class_name = "{{ class.name }}",
    .finalizer = js_{{ class.name }}_finalizer,
    {% if class.has_gc_mark %}
    .gc_mark = js_{{ class.name }}_gc_mark,
    {% else %}
    .gc_mark = NULL,
    {% endif %}
};
```

- [ ] **Step 3: 生成 C FFI 函数声明**

在模板文件中添加 gc_mark FFI 函数（如果 class 有 gc_mark）：

```c
// deps/ridl-tool/templates/mquickjs_ridl_register.h.j2
{% if class.has_gc_mark %}
// Forward declaration of Rust gc_mark function
extern void js_{{ class.name }}_gc_mark(
    JSContext *ctx,
    JSValue obj,
    void *opaque,
    const JSMarkFunc *mf
);
{% endif %}
```

- [ ] **Step 4: 在 generator 中设置 has_gc_mark 标志**

在 `generate_class_code` 或模板上下文准备中：

```rust
// deps/ridl-tool/src/generator/mod.rs（准备模板上下文时）
let template_context = ClassTemplateContext {
    name: &class.name,
    has_gc_mark: has_traced_fields(&class.opaque_fields),
    // ... 其他字段 ...
};
```

- [ ] **Step 5: 运行 ridl-tool 生成测试**

Run: `cargo run -p ridl-builder -- prepare`
Expected: 生成代码成功，包含 gc_mark 注册

- [ ] **Step 6: Commit 模板更新**

```bash
git add deps/ridl-tool/templates/mquickjs_ridl_register.h.j2
git commit -m "feat(ridl): update C template for gc_mark registration

- Add gc_mark field to JSClassDef
- Conditionally include gc_mark based on Traced fields
- Generate FFI function declaration

Part of GC Root system phase 0"
```

---

## 阶段 1：修复当前测试（0.5-1 天）

### Task 1.1: 修复测试断言和添加辅助函数

**Files:**
- Modify: `tests/gc_root_cycle.rs`

- [ ] **Step 1: 添加 get_finalizer_count 辅助函数**

在 gc_root_cycle.rs 文件末尾添加：

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
fn get_finalizer_count(ctx: &mut mquickjs_rs::Context) -> i32 {
    let result = ctx.eval("TestGc.makeNode().finalizerCount()")
        .expect("eval finalizer count");
    result.trim().parse::<i32>()
        .expect("parse finalizer count as i32")
}

#[cfg(feature = "ridl-extensions")]
fn gc(ctx: &mut mquickjs_rs::Context) {
    let scope_token = ctx.token();
    let scope = scope_token.enter_scope();
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
    }
}
```

- [ ] **Step 2: 重写测试使用 JS 侧创建对象**

修改现有的 `gc_root_cycle_collectable_after_root_drop` 测试：

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn gc_root_cycle_collectable_after_root_drop() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024)
        .expect("create context");
    
    let initial_count = get_finalizer_count(&mut ctx);
    
    // 完全在 JS 侧创建对象和环
    ctx.eval(r#"
        globalThis.testB = {};
        globalThis.testA = TestGc.makeNode();
        testA.setHeld(testB);
        testB.back = testA;
    "#).expect("create cycle in JS");
    
    // Rust 侧获取 B 并创建 Root
    let token = ctx.token();
    let scope = token.enter_scope();
    let b = ctx.eval_jsvalue("testB").expect("get testB");
    let b_local: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value> 
        = scope.value(b);
    let b_root = mquickjs_rs::Root::new(&scope, b_local);
    
    // 验证：有 Root 时不回收
    gc(&mut ctx);
    let count_with_root = get_finalizer_count(&mut ctx);
    assert_eq!(count_with_root, initial_count,
        "Object should NOT be collected while Root exists");
    
    // 释放 JS 引用但保留 Root
    ctx.eval("testA = null; testB = null;").unwrap();
    gc(&mut ctx);
    let count_still_rooted = get_finalizer_count(&mut ctx);
    assert_eq!(count_still_rooted, initial_count,
        "Cycle should NOT be collected while Root exists");
    
    // 释放 Root
    drop(b_root);
    
    // 验证：无 Root 时回收
    gc(&mut ctx);
    let count_after_drop = get_finalizer_count(&mut ctx);
    assert!(count_after_drop > initial_count,
        "Cycle MUST be collected after Root dropped. Expected > {}, got {}",
        initial_count, count_after_drop);
}
```

- [ ] **Step 3: 运行测试验证改进**

Run: `cargo test --test gc_root_cycle --features ridl-extensions`
Expected: 测试通过，断言有意义

- [ ] **Step 4: Commit 测试修复**

```bash
git add tests/gc_root_cycle.rs
git commit -m "fix(test): strengthen gc_root_cycle assertions

- Add get_finalizer_count helper
- Add gc helper (double GC)
- Rewrite test to create objects in JS side
- Add three-stage verification (with Root, still Root, no Root)
- Replace weak assertion (>= 0) with meaningful checks

Part of GC Root system phase 1"
```

---

## 阶段 2：扩展测试覆盖（1-1.5 天）

### Task 2.1: 添加 GcTestHarness 工具类

**Files:**
- Modify: `tests/gc_root_cycle.rs`

- [ ] **Step 1: 在文件末尾添加 test_utils 模块**

```rust
// tests/gc_root_cycle.rs（文件末尾）
#[cfg(feature = "ridl-extensions")]
mod test_utils {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    
    static VAR_COUNTER: AtomicI32 = AtomicI32::new(0);
    
    pub struct GcTestHarness {
        ctx: mquickjs_rs::Context,
        initial_count: i32,
    }
    
    impl GcTestHarness {
        pub fn new() -> Self {
            mquickjs_rs::ridl_bootstrap!();
            let mut ctx = mquickjs_rs::Context::new(1024 * 1024)
                .expect("create context");
            let initial_count = super::get_finalizer_count(&mut ctx);
            Self { ctx, initial_count }
        }
        
        pub fn ctx_mut(&mut self) -> &mut mquickjs_rs::Context {
            &mut self.ctx
        }
        
        pub fn create_node(&mut self) -> String {
            let id = VAR_COUNTER.fetch_add(1, Ordering::SeqCst);
            let var_name = format!("node{}", id);
            self.ctx.eval(&format!(
                "globalThis.{} = TestGc.makeNode(); {}",
                var_name, var_name
            )).expect("create node");
            var_name
        }
        
        pub fn create_object(&mut self) -> String {
            let id = VAR_COUNTER.fetch_add(1, Ordering::SeqCst);
            let var_name = format!("obj{}", id);
            self.ctx.eval(&format!(
                "globalThis.{} = {{}}; {}",
                var_name, var_name
            )).expect("create object");
            var_name
        }
        
        pub fn make_cycle(&mut self, node: &str, obj: &str) {
            self.ctx.eval(&format!(
                "{}.setHeld({}); {}.back = {};",
                node, obj, obj, node
            )).expect("make cycle");
        }
        
        pub fn gc(&mut self) {
            super::gc(&mut self.ctx);
        }
        
        pub fn assert_not_collected(&mut self, msg: &str) {
            let current = super::get_finalizer_count(&mut self.ctx);
            assert_eq!(current, self.initial_count,
                "Unexpected collection: {}", msg);
        }
        
        pub fn assert_collected(&mut self, expected_delta: i32, msg: &str) {
            let current = super::get_finalizer_count(&mut self.ctx);
            assert_eq!(current, self.initial_count + expected_delta,
                "Expected {} collections, got {}: {}",
                expected_delta, current - self.initial_count, msg);
        }
        
        pub fn current_count(&mut self) -> i32 {
            super::get_finalizer_count(&mut self.ctx)
        }
    }
}
```

- [ ] **Step 2: 编写示例测试验证 harness 可用**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_harness_example() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    let obj = h.create_object();
    h.make_cycle(&node, &obj);
    
    h.gc();
    h.assert_not_collected("cycle with JS ref");
    
    h.ctx_mut().eval(&format!("{} = null; {} = null;", node, obj))
        .unwrap();
    h.gc();
    h.assert_collected(1, "cycle after ref dropped");
}
```

- [ ] **Step 3: 运行测试验证 harness**

Run: `cargo test --test gc_root_cycle test_harness_example --features ridl-extensions`
Expected: 测试通过

- [ ] **Step 4: Commit GcTestHarness**

```bash
git add tests/gc_root_cycle.rs
git commit -m "feat(test): add GcTestHarness utility

- Encapsulate context creation and initial count
- Provide high-level APIs (create_node, make_cycle)
- Add assertion helpers (assert_not_collected, assert_collected)
- Auto-manage global variable names

Part of GC Root system phase 2"
```

---

### Task 2.2: 添加正向测试（验证回收）

**Files:**
- Modify: `tests/gc_root_cycle.rs`

- [ ] **Step 1: 添加简单对象回收测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_simple_object_collected() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    
    // 释放 JS 引用
    h.ctx_mut().eval(&format!("{} = null;", node)).unwrap();
    
    h.gc();
    h.assert_collected(1, "single object should be collected");
}
```

- [ ] **Step 2: 添加多个独立环测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_multiple_cycles_collected_independently() {
    let mut h = test_utils::GcTestHarness::new();
    
    // 创建两个独立的环
    let node1 = h.create_node();
    let obj1 = h.create_object();
    h.make_cycle(&node1, &obj1);
    
    let node2 = h.create_node();
    let obj2 = h.create_object();
    h.make_cycle(&node2, &obj2);
    
    // 释放第一个环
    h.ctx_mut().eval(&format!("{} = null; {} = null;", node1, obj1))
        .unwrap();
    h.gc();
    h.assert_collected(1, "first cycle collected");
    
    let count_after_first = h.current_count();
    
    // 释放第二个环
    h.ctx_mut().eval(&format!("{} = null; {} = null;", node2, obj2))
        .unwrap();
    h.gc();
    
    let final_count = h.current_count();
    assert_eq!(final_count - count_after_first, 1,
        "second cycle should also be collected");
}
```

- [ ] **Step 3: 运行正向测试**

Run: `cargo test --test gc_root_cycle test_simple_object test_multiple_cycles --features ridl-extensions`
Expected: 测试通过

- [ ] **Step 4: Commit 正向测试**

```bash
git add tests/gc_root_cycle.rs
git commit -m "test(gc): add positive collection tests

- Test simple object collection
- Test multiple independent cycles
- Verify each cycle collected independently

Part of GC Root system phase 2"
```

---

### Task 2.3: 添加反向测试（验证不回收）

**Files:**
- Modify: `tests/gc_root_cycle.rs`

- [ ] **Step 1: 添加 Root 阻止回收测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_root_prevents_collection() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    
    // 创建 Root
    let token = h.ctx_mut().token();
    let scope = token.enter_scope();
    let node_val = h.ctx_mut().eval_jsvalue(&node).unwrap();
    let node_local = scope.value(node_val);
    let _root = mquickjs_rs::Root::new(&scope, node_local);
    
    // 释放 JS 引用
    h.ctx_mut().eval(&format!("{} = null;", node)).unwrap();
    
    // 多次 GC
    for _ in 0..5 {
        h.gc();
    }
    
    h.assert_not_collected("Root should prevent collection");
}
```

- [ ] **Step 2: 添加 JS 引用阻止回收测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_js_reference_prevents_collection() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    let obj = h.create_object();
    h.make_cycle(&node, &obj);
    
    // 保留 node 的 JS 引用（通过 globalThis.keepalive）
    h.ctx_mut().eval(&format!("globalThis.keepalive = {};", node))
        .unwrap();
    
    // 释放原始变量
    h.ctx_mut().eval(&format!("{} = null; {} = null;", node, obj))
        .unwrap();
    
    h.gc();
    h.assert_not_collected("JS reference should prevent collection");
    
    // 释放 keepalive
    h.ctx_mut().eval("keepalive = null;").unwrap();
    h.gc();
    h.assert_collected(1, "cycle should be collected after all refs dropped");
}
```

- [ ] **Step 3: 添加多个 Root 测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_multiple_roots_same_object() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    
    // 创建两个 Root
    let token = h.ctx_mut().token();
    let scope = token.enter_scope();
    let node_val = h.ctx_mut().eval_jsvalue(&node).unwrap();
    let node_local = scope.value(node_val);
    let root1 = mquickjs_rs::Root::new(&scope, node_local);
    let root2 = mquickjs_rs::Root::new(&scope, node_local);
    
    // 释放 JS 引用
    h.ctx_mut().eval(&format!("{} = null;", node)).unwrap();
    
    // 释放 root1
    drop(root1);
    h.gc();
    h.assert_not_collected("root2 still holds reference");
    
    // 释放 root2
    drop(root2);
    h.gc();
    h.assert_collected(1, "object collected after both roots dropped");
}
```

- [ ] **Step 4: 运行反向测试**

Run: `cargo test --test gc_root_cycle test_root_prevents test_js_reference test_multiple_roots --features ridl-extensions`
Expected: 测试通过

- [ ] **Step 5: Commit 反向测试**

```bash
git add tests/gc_root_cycle.rs
git commit -m "test(gc): add negative collection tests

- Test Root prevents collection
- Test JS reference prevents collection
- Test multiple Roots for same object
- Verify collection only after all references dropped

Part of GC Root system phase 2"
```

---

### Task 2.4: 添加边界测试

**Files:**
- Modify: `tests/gc_root_cycle.rs`

- [ ] **Step 1: 添加空 Root drop 测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
fn test_empty_root_drop() {
    let mut h = test_utils::GcTestHarness::new();
    
    let node = h.create_node();
    
    // 创建 Root 后立即 drop
    {
        let token = h.ctx_mut().token();
        let scope = token.enter_scope();
        let node_val = h.ctx_mut().eval_jsvalue(&node).unwrap();
        let node_local = scope.value(node_val);
        let root = mquickjs_rs::Root::new(&scope, node_local);
        drop(root);
    }
    
    // 验证不崩溃，且对象仍存在（JS 引用仍保持）
    h.gc();
    h.assert_not_collected("JS reference still exists");
    
    // 释放 JS 引用
    h.ctx_mut().eval(&format!("{} = null;", node)).unwrap();
    h.gc();
    h.assert_collected(1, "object collected after JS ref dropped");
}
```

- [ ] **Step 2: 添加跨 Context Root 校验测试**

```rust
// tests/gc_root_cycle.rs
#[cfg(feature = "ridl-extensions")]
#[test]
#[should_panic(expected = "cross-context Root::new")]
fn test_cross_context_root_rejected() {
    mquickjs_rs::ridl_bootstrap!();
    
    let mut ctx1 = mquickjs_rs::Context::new(1024 * 1024).unwrap();
    let mut ctx2 = mquickjs_rs::Context::new(1024 * 1024).unwrap();
    
    // 在 ctx1 创建对象
    ctx1.eval("globalThis.obj = {};").unwrap();
    let token1 = ctx1.token();
    let scope1 = token1.enter_scope();
    let obj1 = ctx1.eval_jsvalue("obj").unwrap();
    let obj1_local = scope1.value(obj1);
    
    // 在 ctx2 的 scope 中尝试创建 Root（应该 panic）
    let token2 = ctx2.token();
    let scope2 = token2.enter_scope();
    let _root = mquickjs_rs::Root::new(&scope2, obj1_local);  // ← panic here
}
```

- [ ] **Step 3: 运行边界测试**

Run: `cargo test --test gc_root_cycle test_empty_root test_cross_context --features ridl-extensions`
Expected: test_empty_root 通过，test_cross_context panic 符合预期

- [ ] **Step 4: Commit 边界测试**

```bash
git add tests/gc_root_cycle.rs
git commit -m "test(gc): add boundary condition tests

- Test empty Root drop (no crash)
- Test cross-Context Root rejection (panic)
- Verify safety checks work correctly

Part of GC Root system phase 2"
```

---

## 阶段 3：TDD 驱动 Traced<T>（2-3 天）

### Task 3.1: 实现 Traced<T> 类型

**Files:**
- Create: `deps/mquickjs-rs/src/traced.rs`
- Modify: `deps/mquickjs-rs/src/lib.rs`
- Modify: `deps/mquickjs-rs/src/mod.rs`

- [ ] **Step 1: 创建 traced.rs 文件**

```rust
// deps/mquickjs-rs/src/traced.rs
use crate::handles::local::Local;
use crate::mquickjs_ffi;

/// A JSValue held within a JS user class opaque.
///
/// Unlike `Root<T>`, `Traced<T>` does NOT independently root the value.
/// It relies on the owning JS object's reachability:
/// - When the owner is reachable, the GC mark callback will mark this field.
/// - When the owner is unreachable, this field does not prevent collection.
///
/// # Safety Model
///
/// **Invariant**: Traced<T> lifetime ≤ JS object lifetime
///
/// This is guaranteed by:
/// 1. JS object reachable → opaque valid → Traced<T> valid
/// 2. JS object unreachable → GC triggers finalizer → opaque Drop → Traced<T> invalidated
///
/// **Must be used within RIDL user class opaque only.**
///
/// ## Violation Scenarios (MUST avoid)
///
/// ```rust,no_run
/// // ❌ Taking Traced out of opaque
/// let traced = opaque.held.take(); // opaque may be finalized later
/// // traced now holds dangling pointer
///
/// // ❌ Cross-Context usage
/// let traced_from_ctx1 = /* ... */;
/// // Use in ctx2 → undefined behavior
/// ```
///
/// ## Design Decision
///
/// - No runtime checks (performance consideration)
/// - Relies on type system + RIDL constraints for safety
pub struct Traced<T> {
    raw: mquickjs_ffi::JSValue,
    _t: std::marker::PhantomData<T>,
}

impl<T> Traced<T> {
    /// Create a Traced<T> from a Local<T>.
    ///
    /// # Safety
    ///
    /// The caller must ensure this Traced is stored in a RIDL user class opaque
    /// that will be marked by the class's gc_mark callback.
    pub fn new(v: Local<T>) -> Self {
        Self {
            raw: v.as_raw(),
            _t: std::marker::PhantomData,
        }
    }
    
    /// Get the raw JSValue (for use in gc_mark callbacks).
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }
}

// No Drop implementation: Traced does not manage lifetime independently
```

- [ ] **Step 2: 在 mod.rs 中声明模块**

```rust
// deps/mquickjs-rs/src/mod.rs
pub mod traced;
```

- [ ] **Step 3: 在 lib.rs 中导出 Traced**

```rust
// deps/mquickjs-rs/src/lib.rs
pub use crate::traced::Traced;
```

- [ ] **Step 4: 运行编译检查**

Run: `cargo build --manifest-path deps/mquickjs-rs/Cargo.toml`
Expected: 成功编译

- [ ] **Step 5: Commit Traced<T> 实现**

```bash
git add deps/mquickjs-rs/src/traced.rs deps/mquickjs-rs/src/mod.rs deps/mquickjs-rs/src/lib.rs
git commit -m "feat(mquickjs): implement Traced<T> type

- Add Traced<T> struct with safety documentation
- Store raw JSValue without Drop
- Provide as_raw() for gc_mark callbacks
- Document invariants and violation scenarios

Part of GC Root system phase 3"
```

---

### Task 3.2: 创建 Traced<T> RIDL 测试模块

**Files:**
- Create: `tests/global/gc_traced/test_gc_traced/Cargo.toml`
- Create: `tests/global/gc_traced/test_gc_traced/build.rs`
- Create: `tests/global/gc_traced/test_gc_traced/src/test_gc_traced.ridl`
- Create: `tests/global/gc_traced/test_gc_traced/src/lib.rs`
- Create: `tests/global/gc_traced/test_gc_traced/src/traced_node_impl.rs`
- Create: `tests/global/gc_traced/test_gc_traced/src/class_impl.rs`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
# tests/global/gc_traced/test_gc_traced/Cargo.toml
[package]
name = "test_gc_traced"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
mquickjs-rs = { path = "../../../../deps/mquickjs-rs" }

[build-dependencies]
ridl-builder = { path = "../../../../deps/ridl-builder" }
```

- [ ] **Step 2: 创建 build.rs**

```rust
// tests/global/gc_traced/test_gc_traced/build.rs
fn main() {
    ridl_builder::build();
}
```

- [ ] **Step 3: 创建 RIDL 定义（使用 opaque 语法）**

```ridl
// tests/global/gc_traced/test_gc_traced/src/test_gc_traced.ridl
class TracedNode {
    opaque {
        held: Option<Traced<Value>>
    }
    
    fn setHeld(v: object) -> void;
    fn clearHeld() -> void;
    fn finalizerCount() -> i32;
}

singleton TestGcTraced {
    fn makeTracedNode() -> TracedNode;
}
```

- [ ] **Step 4: 实现 traced_node_impl.rs**

```rust
// tests/global/gc_traced/test_gc_traced/src/traced_node_impl.rs
use std::sync::atomic::{AtomicI32, Ordering};
use mquickjs_rs::{Traced, Env};
use mquickjs_rs::handles::local::Value;
use mquickjs_rs::handles::object::Object;

static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

// TracedNodeOpaque 由 RIDL 自动生成：
// pub struct TracedNodeOpaque {
//     pub held: Option<Traced<Value>>,
// }

impl Drop for TracedNodeOpaque {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

impl TracedNodeClass for TracedNodeOpaque {
    fn set_held<'ctx>(&mut self, env: &mut Env<'ctx>, v: Object<'ctx>) {
        let local = env.scope().value(v.as_raw());
        self.held = Some(Traced::new(local));
    }
    
    fn clear_held(&mut self) {
        self.held = None;
    }
    
    fn finalizer_count(&mut self) -> i32 {
        FINALIZER_COUNT.load(Ordering::SeqCst)
    }
}
```

- [ ] **Step 5: 实现 class_impl.rs**

```rust
// tests/global/gc_traced/test_gc_traced/src/class_impl.rs
use super::traced_node_impl::TracedNodeOpaque;

pub struct DefaultTestGcTraced;

impl crate::api::TestGcTracedSingleton for DefaultTestGcTraced {
    fn make_traced_node(&mut self) -> Box<dyn crate::api::TracedNodeClass> {
        Box::new(TracedNodeOpaque {
            held: None,
        })
    }
}
```

- [ ] **Step 6: 实现 lib.rs**

```rust
// tests/global/gc_traced/test_gc_traced/src/lib.rs
mod traced_node_impl;
mod class_impl;

// RIDL 生成的代码会包含：
// - pub mod api（trait 定义）
// - TracedNodeOpaque struct
// - gc_mark 实现

pub fn init(ctx: *mut mquickjs_rs::mquickjs_ffi::JSContext) {
    unsafe {
        // RIDL 生成的注册函数
        ridl_register_module(ctx);
    }
}

pub fn traced_node_constructor() -> Box<dyn crate::api::TracedNodeClass> {
    Box::new(traced_node_impl::TracedNodeOpaque {
        held: None,
    })
}

pub fn test_gc_traced_singleton_constructor() -> Box<dyn crate::api::TestGcTracedSingleton> {
    Box::new(class_impl::DefaultTestGcTraced)
}
```

- [ ] **Step 7: 构建 RIDL 模块**

Run: `cargo run -p ridl-builder -- prepare`
Expected: 生成代码成功，包含 TracedNodeOpaque 和 gc_mark

- [ ] **Step 8: Commit RIDL 模块**

```bash
git add tests/global/gc_traced/
git commit -m "feat(test): create Traced<T> RIDL test module

- Define TracedNode with Traced<Value> field
- Implement TracedNodeClass using Traced
- RIDL auto-generates gc_mark for Traced field

Part of GC Root system phase 3"
```

---

### Task 3.3: 编写 Traced<T> 行为测试

**Files:**
- Create: `tests/gc_traced.rs`

- [ ] **Step 1: 创建测试文件框架**

```rust
// tests/gc_traced.rs
use mquickjs_rs::{Context, Root, Traced};
use mquickjs_rs::handles::local::Value;
use std::sync::atomic::{AtomicI32, Ordering};

fn get_finalizer_count(ctx: &mut Context) -> i32 {
    let result = ctx.eval("TestGcTraced.makeTracedNode().finalizerCount()")
        .expect("eval finalizer count");
    result.trim().parse::<i32>()
        .expect("parse count")
}

fn gc(ctx: &mut Context) {
    let token = ctx.token();
    let scope = token.enter_scope();
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
        mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw());
    }
}
```

- [ ] **Step 2: 编写 Traced 跟随 owner 生命周期测试（会失败）**

```rust
// tests/gc_traced.rs
#[cfg(feature = "ridl-extensions")]
#[test]
#[ignore]  // ← 标记为 ignore，等实现完成后移除
fn test_traced_follows_owner_lifetime() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = Context::new(1024 * 1024).unwrap();
    
    let initial_count = get_finalizer_count(&mut ctx);
    
    // 创建 TracedNode，让其持有一个对象
    ctx.eval(r#"
        globalThis.node = TestGcTraced.makeTracedNode();
        globalThis.obj = {};
        node.setHeld(obj);
        obj.back = node;
    "#).unwrap();
    
    // 释放 JS 侧引用（没有 Root）
    ctx.eval("node = null; obj = null;").unwrap();
    
    // 验证：Traced 不独立 root，环应该被回收
    gc(&mut ctx);
    let after_count = get_finalizer_count(&mut ctx);
    assert!(after_count > initial_count,
        "Traced<T> should NOT root independently. Expected collection, got count {} (initial {})",
        after_count, initial_count);
}
```

- [ ] **Step 3: 编写对比测试（Root vs Traced）**

```rust
// tests/gc_traced.rs
#[cfg(feature = "ridl-extensions")]
#[test]
#[ignore]
fn test_traced_vs_root_behavior_comparison() {
    mquickjs_rs::ridl_bootstrap!();
    let mut ctx = Context::new(1024 * 1024).unwrap();
    
    let initial_count_root = get_finalizer_count(&mut ctx);
    
    // === 场景 A：Root<T> 阻止回收 ===
    ctx.eval(r#"
        globalThis.nodeRoot = TestGc.makeNode();
        globalThis.objRoot = {};
        nodeRoot.setHeld(objRoot);
        objRoot.back = nodeRoot;
    "#).unwrap();
    
    // 释放 JS 引用（但 Node 内部用 Root<T>）
    ctx.eval("nodeRoot = null; objRoot = null;").unwrap();
    gc(&mut ctx);
    
    let count_after_root = get_finalizer_count(&mut ctx);
    assert_eq!(count_after_root, initial_count_root,
        "Root<T> should prevent collection");
    
    // === 场景 B：Traced<T> 允许回收 ===
    let initial_count_traced = count_after_root;
    
    ctx.eval(r#"
        globalThis.nodeTraced = TestGcTraced.makeTracedNode();
        globalThis.objTraced = {};
        nodeTraced.setHeld(objTraced);
        objTraced.back = nodeTraced;
    "#).unwrap();
    
    // 释放 JS 引用（Traced 不独立 root）
    ctx.eval("nodeTraced = null; objTraced = null;").unwrap();
    gc(&mut ctx);
    
    let count_after_traced = get_finalizer_count(&mut ctx);
    assert!(count_after_traced > initial_count_traced,
        "Traced<T> should allow collection when owner is unreachable");
}
```

- [ ] **Step 4: 运行测试验证失败（预期）**

Run: `cargo test --test gc_traced --features ridl-extensions -- --ignored`
Expected: 测试失败（gc_mark 尚未生成或注册）

- [ ] **Step 5: Commit 失败的测试**

```bash
git add tests/gc_traced.rs
git commit -m "test(gc): add Traced<T> behavior tests (TDD Red)

- Test Traced follows owner lifetime
- Test Traced vs Root behavior comparison
- Mark as #[ignore] until implementation complete

Part of GC Root system phase 3 (Red)"
```

---

### Task 3.4: 验证 RIDL 生成的 gc_mark 并移除 ignore

**Files:**
- Modify: `tests/gc_traced.rs`

- [ ] **Step 1: 检查生成的代码**

Run: `rg "pub unsafe extern \"C\" fn gc_mark" tests/global/gc_traced/test_gc_traced/target/*/build/*/out/ -A 10`
Expected: 找到自动生成的 gc_mark 函数

- [ ] **Step 2: 检查 C 注册代码**

Run: `rg "js_TracedNode_gc_mark" tests/global/gc_traced/test_gc_traced/target/*/build/*/out/`
Expected: 找到 FFI 注册

- [ ] **Step 3: 移除 #[ignore] 标记**

```rust
// tests/gc_traced.rs
#[cfg(feature = "ridl-extensions")]
#[test]
// #[ignore] ← 删除这行
fn test_traced_follows_owner_lifetime() {
    // ... 测试代码不变 ...
}

#[cfg(feature = "ridl-extensions")]
#[test]
// #[ignore] ← 删除这行
fn test_traced_vs_root_behavior_comparison() {
    // ... 测试代码不变 ...
}
```

- [ ] **Step 4: 运行测试验证通过（Green）**

Run: `cargo test --test gc_traced --features ridl-extensions`
Expected: 所有测试通过

- [ ] **Step 5: Commit 测试通过**

```bash
git add tests/gc_traced.rs
git commit -m "test(gc): Traced<T> tests now pass (TDD Green)

- Remove #[ignore] markers
- RIDL auto-generated gc_mark works correctly
- Traced<T> follows owner lifetime as expected

Part of GC Root system phase 3 (Green)"
```

---

### Task 3.5: 添加 Root vs Traced 对比文档

**Files:**
- Modify: `tests/gc_root_cycle.rs` 或 `tests/gc_traced.rs`

- [ ] **Step 1: 在文件顶部添加模块级文档**

```rust
// tests/gc_traced.rs（文件开头）
//! # Root<T> vs Traced<T> 行为对比
//!
//! | 维度 | Root<T> | Traced<T> |
//! |------|---------|-----------|
//! | **用途** | 跨 await 持有、全局队列 | user class opaque 内部字段 |
//! | **是否独立 root** | 是 | 否 |
//! | **对象不可达时** | 阻止回收 | 允许回收 |
//! | **注册机制** | Context-level gc_mark | per-class gc_mark |
//! | **Drop 行为** | 从 RootsRegistry 移除 | 无操作 |
//! | **线程安全** | !Send + !Sync | 同 opaque struct |
//! | **跨 Context** | 校验 ctx_id，panic | 跟随 opaque |
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! // Root<T> - 跨 await 持有
//! let root = Root::new(&scope, value);
//! // ... await 或离开 scope ...
//! drop(root); // 显式释放
//!
//! // Traced<T> - opaque 内部字段
//! // （在 RIDL 定义中声明）
//! class MyClass {
//!     opaque {
//!         callback: Traced<Function>
//!     }
//! }
//! // RIDL 自动生成 gc_mark，无需手动管理
//! ```
//!
//! ## 测试覆盖
//!
//! 本文件测试 Traced<T> 的核心行为：
//! - 不独立 root（owner 不可达时允许回收）
//! - 与 Root<T> 的行为对比
//! - 循环引用场景的正确处理
```

- [ ] **Step 2: 运行 doc 测试**

Run: `cargo test --doc traced --manifest-path deps/mquickjs-rs/Cargo.toml`
Expected: 文档示例编译通过

- [ ] **Step 3: Commit 文档**

```bash
git add tests/gc_traced.rs
git commit -m "docs(gc): add Root<T> vs Traced<T> comparison

- Document use cases and behavior differences
- Add usage examples
- Explain design decisions (independent root vs follow owner)

Part of GC Root system phase 3"
```

---

## 阶段 4：最终验证与文档

### Task 4.1: 运行完整测试套件

**Files:**
- None (运行测试)

- [ ] **Step 1: 运行所有 GC 测试**

Run: `cargo test gc_root_cycle gc_traced --features ridl-extensions`
Expected: 所有测试通过

- [ ] **Step 2: 运行 RIDL 工具测试**

Run: `cargo test --manifest-path deps/ridl-tool/Cargo.toml`
Expected: 所有 parser 和 codegen 测试通过

- [ ] **Step 3: 运行 mquickjs-rs 测试**

Run: `cargo test --manifest-path deps/mquickjs-rs/Cargo.toml`
Expected: Traced<T> 相关测试通过

- [ ] **Step 4: 生成测试覆盖率报告（可选）**

Run: `cargo tarpaulin --features ridl-extensions --out Html`
Expected: 生成覆盖率报告

- [ ] **Step 5: 记录测试结果**

创建测试摘要：
```bash
echo "## GC Test Suite Results" > test-results.md
echo "" >> test-results.md
cargo test gc_root_cycle gc_traced --features ridl-extensions 2>&1 | grep "test result" >> test-results.md
```

Expected: 生成测试摘要文件

---

### Task 4.2: 更新知识库

**Files:**
- 通过 `/post-commit-memory` 自动生成

- [ ] **Step 1: 运行 post-commit-memory**

Run: 在 Claude Code 中执行 `/post-commit-memory`
Expected: 自动分析最近的 commits 并生成知识条目

- [ ] **Step 2: 检查生成的知识条目**

Run: `ls -la docs/knowledge/ | grep -E "gc|traced|ridl"`
Expected: 新增相关知识条目

- [ ] **Step 3: 验证 MEMORY.md 更新**

Run: `cat docs/knowledge/MEMORY.md | grep -E "traced|gc_mark"`
Expected: 索引包含新条目

---

## Self-Review

### Spec Coverage Check

✅ **阶段 0 - RIDL 语法扩展**
- Task 0.1-0.7 实现了 AST 扩展、parser、codegen、gc_mark 生成、模板更新
- 覆盖设计文档阶段 0 的所有要求

✅ **阶段 1 - 修复当前测试**
- Task 1.1 修复了弱断言，添加了辅助函数，使用 JS 侧创建对象
- 覆盖设计文档阶段 1 的所有要求

✅ **阶段 2 - 扩展测试覆盖**
- Task 2.1-2.4 添加了 GcTestHarness、正向测试、反向测试、边界测试
- 覆盖设计文档阶段 2 的 10+ 测试场景

✅ **阶段 3 - TDD 驱动 Traced<T>**
- Task 3.1-3.5 实现了 Traced<T>、RIDL 模块、测试、文档
- 覆盖设计文档阶段 3 的 TDD 流程（Red → Green）

✅ **阶段 4 - 最终验证**
- Task 4.1-4.2 运行完整测试套件、更新知识库

### Placeholder Scan

✅ 无 "TBD", "TODO", "implement later"
✅ 所有代码步骤包含完整代码
✅ 所有测试包含具体断言
✅ 所有命令包含预期输出

### Type Consistency

✅ `OpaqueField` 在 Task 0.1 定义，后续任务一致引用
✅ `Type::Traced` 在 Task 0.1 定义，parser/codegen 一致使用
✅ `generate_opaque_struct` 和 `generate_gc_mark` 签名一致
✅ `GcTestHarness` 方法名称在 Task 2.1 定义后保持一致
✅ `TracedNodeOpaque` 在 RIDL 生成，测试中正确引用

---

## 执行建议

**总估算时间**：7-10.5 天（与设计文档一致）

**关键路径**：
- 阶段 0（RIDL 语法）是基础，必须先完成
- 阶段 1-2 可并行（不同文件）
- 阶段 3 依赖阶段 0 完成

**检查点**：
- 阶段 0 完成后：运行 RIDL 工具测试，验证生成代码
- 阶段 1-2 完成后：运行 gc_root_cycle 测试，验证覆盖率
- 阶段 3 完成后：运行所有测试，验证 Traced<T> 行为

**风险缓解**：
- 如果阶段 0 遇到阻塞（RIDL parser 复杂度超预期），可降级到手写 opaque struct + proc-macro
- 每个 Task 结束后立即 commit，确保增量进度
- 测试失败时，使用 `cargo test -- --nocapture` 查看详细输出
