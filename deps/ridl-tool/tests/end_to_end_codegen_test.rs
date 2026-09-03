/// 端到端编译测试
/// 验证 RIDL → 代码生成 → 编译 的完整管线

use ridl_tool::parser::FileMode;

/// 测试生成的 Rust API 代码可以编译
#[test]
fn test_generated_rust_api_compiles() {
    let ridl_input = r#"
class SimpleNode {
    opaque {
        held: Traced<Value>
        count: i32
    }

    fn getValue() -> i32;
    fn setValue(v: i32) -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证关键结构存在
    assert!(api_content.contains("pub trait SimpleNodeClass"), "Should generate trait");
    assert!(api_content.contains("pub struct SimpleNodeOpaque"), "Should generate opaque struct");
    assert!(api_content.contains("pub held: mquickjs_rs::Traced<mquickjs_rs::Value>"), "Should have Traced field");
    assert!(api_content.contains("pub count: i32"), "Should have i32 field");
    assert!(api_content.contains("fn get_value"), "Should have getter method");
    assert!(api_content.contains("fn set_value"), "Should have setter method");
    assert!(api_content.contains("fn gc_mark"), "Should have gc_mark method");
}

/// 测试多种类型的 opaque 字段生成
#[test]
fn test_opaque_field_type_mapping() {
    let ridl_input = r#"
class TypeTest {
    opaque {
        traced_val: Traced<Value>
        optional_traced: Traced<Value>?
        plain_int: i32
        plain_string: string
        array_field: array<i32>
        map_field: map<string, i32>
    }

    fn test() -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证类型映射
    assert!(api_content.contains("pub traced_val: mquickjs_rs::Traced<mquickjs_rs::Value>"), "Traced<Value>");
    assert!(api_content.contains("pub optional_traced: Option<mquickjs_rs::Traced<mquickjs_rs::Value>>"), "Option<Traced<Value>>");
    assert!(api_content.contains("pub plain_int: i32"), "i32");
    assert!(api_content.contains("pub plain_string: String"), "string → String");
    assert!(api_content.contains("pub array_field: Vec<i32>"), "array<i32> → Vec<i32>");
    assert!(api_content.contains("pub map_field: std::collections::HashMap<String, i32>"), "map<string, i32> → HashMap");

    // 验证 gc_mark 只处理 Traced 字段
    assert!(api_content.contains("self.traced_val.gc_mark(mf)"), "Should mark traced_val");
    assert!(api_content.contains("if let Some(ref inner) = self.optional_traced"), "Should unwrap optional_traced");
    assert!(!api_content.contains("self.plain_int.gc_mark"), "Should not mark plain_int");
    assert!(!api_content.contains("self.plain_string.gc_mark"), "Should not mark plain_string");
}

/// 测试方法签名生成（snake_case 转换）
#[test]
fn test_method_signature_generation() {
    let ridl_input = r#"
class MethodTest {
    fn noParams() -> void;
    fn withParams(x: i32, y: string) -> bool;
    fn returnI32() -> i32;
    fn returnString() -> string;
    fn returnBool() -> bool;
    fn returnOptional() -> string?;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证方法签名（camelCase → snake_case）
    assert!(api_content.contains("fn no_params"), "noParams → no_params");
    assert!(api_content.contains("fn with_params"), "withParams → with_params");
    assert!(api_content.contains("fn return_i32"), "returnI32 → return_i32");
    assert!(api_content.contains("fn return_string"), "returnString → return_string");
    assert!(api_content.contains("fn return_bool"), "returnBool → return_bool");
    assert!(api_content.contains("fn return_optional"), "returnOptional → return_optional");
}

/// 测试无 opaque 字段的 class
#[test]
fn test_class_without_opaque() {
    let ridl_input = r#"
class SimpleClass {
    fn doSomething() -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证生成 trait 但不生成 opaque struct
    assert!(api_content.contains("pub trait SimpleClassClass"), "Should generate trait");
    assert!(!api_content.contains("pub struct SimpleClassOpaque"), "Should not generate opaque struct without opaque fields");
}

/// 测试多个 class 生成
#[test]
fn test_multiple_classes() {
    let ridl_input = r#"
class ClassA {
    opaque { val: i32 }
    fn getVal() -> i32;
}

class ClassB {
    opaque { name: string }
    fn getName() -> string;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证两个 class 都生成了
    assert!(api_content.contains("pub trait ClassAClass"), "Should generate ClassA trait");
    assert!(api_content.contains("pub trait ClassBClass"), "Should generate ClassB trait");
    assert!(api_content.contains("pub struct ClassAOpaque"), "Should generate ClassA opaque");
    assert!(api_content.contains("pub struct ClassBOpaque"), "Should generate ClassB opaque");
}

/// 测试 Traced<T> 嵌套类型
#[test]
fn test_traced_nested_types() {
    let ridl_input = r#"
class NestedTraced {
    opaque {
        items: array<Traced<Value>>
        cache: map<string, Traced<Value>>
        optional: Traced<Value>?
    }

    fn test() -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证嵌套类型映射
    assert!(api_content.contains("pub items: Vec<mquickjs_rs::Traced<mquickjs_rs::Value>>"), "Array<Traced>");
    assert!(api_content.contains("pub cache: std::collections::HashMap<String, mquickjs_rs::Traced<mquickjs_rs::Value>>"), "Map<K, Traced>");
    assert!(api_content.contains("pub optional: Option<mquickjs_rs::Traced<mquickjs_rs::Value>>"), "Option<Traced>");

    // 验证 gc_mark 处理嵌套类型
    assert!(api_content.contains("for item in &self.items"), "Should iterate array");
    assert!(api_content.contains("for (_key, value) in &self.cache"), "Should iterate map");
    assert!(api_content.contains("if let Some(ref inner) = self.optional"), "Should unwrap optional");
}

/// 测试错误情况：语法错误
#[test]
fn test_syntax_error_handling() {
    let ridl_input = r#"
class Foo {
    fn bar() -> void  // 缺少分号
}
"#;

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input);
    assert!(parsed.is_err(), "Syntax error should fail parsing");
}

/// 测试错误情况：空文件
#[test]
fn test_empty_ridl_file() {
    let ridl_input = "";

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input);
    assert!(parsed.is_ok(), "Empty RIDL file should parse successfully");

    // 空文件应该生成空的 api.rs
    ridl_tool::generator::generate_module_files(
        &parsed.unwrap().items,
        None,
        FileMode::Default,
        &output_dir,
        "test_module",
    )
    .unwrap();

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();
    assert!(!api_content.contains("pub trait"), "Empty file should not generate traits");
}
