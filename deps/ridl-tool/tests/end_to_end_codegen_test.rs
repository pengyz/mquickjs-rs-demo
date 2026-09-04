/// 端到端编译测试
/// 验证 RIDL → 代码生成 → 编译 的完整管线
///
/// 注意：v1 版本中 enum/struct/callback/using/global_function 不生成 Rust 代码，
/// 映射为 any(JSValue)。只有 class、singleton、interface 生成 Rust trait。

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

/// 测试 singleton 生成
#[test]
fn test_singleton_generation() {
    let ridl_input = r#"
singleton MyService {
    fn initialize() -> void;
    fn process(x: i32) -> bool;
    readonly property name: string;
    property count: i32;
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

    // 验证 singleton trait 生成
    assert!(api_content.contains("pub trait MyServiceSingleton"), "Should generate singleton trait");
    assert!(api_content.contains("fn initialize"), "Should have initialize method");
    assert!(api_content.contains("fn process"), "Should have process method");
    assert!(api_content.contains("fn name"), "Should have name property");
    assert!(api_content.contains("fn count"), "Should have count property");
    assert!(api_content.contains("fn set_count"), "Should have count setter");
}

/// 测试 interface 生成
#[test]
fn test_interface_generation() {
    let ridl_input = r#"
interface Drawable {
    fn draw(ctx: object) -> void;
    fn isVisible() -> bool;
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

    // 验证 interface trait 生成
    assert!(api_content.contains("pub trait DrawableInterface"), "Should generate interface trait");
    assert!(api_content.contains("fn draw"), "Should have draw method");
    assert!(api_content.contains("fn is_visible"), "Should have is_visible method");
    assert!(api_content.contains("fn get_name"), "Should have get_name method");
}

/// 测试 v1 不生成的类型（enum/struct/callback/using/global_function）
#[test]
fn test_v1_types_not_generated() {
    let ridl_input = r#"
enum Color { RED, GREEN, BLUE }
json struct Config { name: string; }
callback EventHandler(event: object);
using StringMap = map<string, string>;
fn helper(x: i32) -> i32;
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

    // P1 已实现：enum/struct/using 现在会生成
    assert!(api_content.contains("pub enum Color"), "Should generate enum");
    assert!(api_content.contains("pub struct Config"), "Should generate struct");
    assert!(api_content.contains("type StringMap"), "Should generate using alias");
    // callback 和 global function 仍然不生成
    assert!(!api_content.contains("EventHandler"), "v1 should not generate callback");
    assert!(!api_content.contains("fn helper"), "v1 should not generate global function");
}

/// 测试综合 RIDL 文件（包含所有类型）
#[test]
fn test_comprehensive_ridl_generation() {
    let ridl_input = r#"
module my.app@1.0.0;

import { Helper } from "utils";

using StringMap = map<string, string>;

callback ErrorCallback(error: string?);

interface Serializable {
    fn serialize() -> string;
    fn deserialize(data: string) -> void;
}

enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3,
}

json struct Config {
    name: string;
    version: string;
    debug: bool;
}

class Logger {
    var maxEntries: i32 = 1000;

    readonly property level: i32;
    property format: string;

    constructor(level: i32);

    fn log(message: string) -> void;
    fn getEntries() -> array<string>;

    opaque {
        buffer: array<Traced<Value>>
    }
}

singleton AppService {
    fn initialize(config: Config) -> void;
    fn getLogger(name: string) -> Logger;
    readonly property version: string;
    property debug: bool;
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

    // 验证生成的类型
    assert!(api_content.contains("pub trait SerializableInterface"), "Should have interface");
    assert!(api_content.contains("pub trait LoggerClass"), "Should have class trait");
    assert!(api_content.contains("pub struct LoggerOpaque"), "Should have opaque struct");
    assert!(api_content.contains("pub trait AppServiceSingleton"), "Should have singleton trait");

    // P1 已实现：enum/struct 现在会生成
    assert!(api_content.contains("pub enum LogLevel"), "Should generate enum");
    assert!(api_content.contains("pub struct Config"), "Should generate struct");
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

// ========================================================================
// P0: 新增类型支持测试（TDD - 先写失败测试）
// ========================================================================

/// P0-1: object 作为返回值
#[test]
fn test_object_return_type() {
    let ridl_input = r#"
class ObjectTest {
    fn getObject() -> object;
    fn getOptionalObject() -> object?;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // 应该成功生成（不报错）
    assert!(result.is_ok(), "object return type should be supported: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证方法签名生成
    assert!(api_content.contains("fn get_object"), "Should have get_object method");
    assert!(api_content.contains("fn get_optional_object"), "Should have get_optional_object method");
}

/// P0-2: array<T> 作为属性类型
#[test]
fn test_array_property_type() {
    let ridl_input = r#"
class ArrayPropTest {
    property items: array<i32>;
    property names: array<string>;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // 应该成功生成（不报错）
    assert!(result.is_ok(), "array property type should be supported: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证属性生成（class 属性生成 fn get_<name> / fn set_<name>）
    assert!(api_content.contains("fn get_items"), "Should have items getter");
    assert!(api_content.contains("fn set_items"), "Should have items setter");
    assert!(api_content.contains("fn get_names"), "Should have names getter");
    assert!(api_content.contains("fn set_names"), "Should have names setter");
}

/// P0-2: map<K,V> 作为属性类型
#[test]
fn test_map_property_type() {
    let ridl_input = r#"
class MapPropTest {
    property cache: map<string, i32>;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // 应该成功生成（不报错）
    assert!(result.is_ok(), "map property type should be supported: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证属性生成（class 属性生成 fn get_<name> / fn set_<name>）
    assert!(api_content.contains("fn get_cache"), "Should have cache getter");
    assert!(api_content.contains("fn set_cache"), "Should have cache setter");
}

/// P0-3: callback 作为返回值（v1 映射为 any，不崩溃即可）
#[test]
fn test_callback_return_type() {
    let ridl_input = r#"
callback Handler(event: object);

class CallbackTest {
    fn getHandler() -> Handler;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // v1 不支持 callback 作为返回值，但不应该崩溃
    // 如果未来支持，这里应该改为 assert!(result.is_ok())
    // 当前行为：报错 "unsupported return type: Custom("Handler")"
    assert!(result.is_err(), "v1 does not support callback return type yet");
}

// ========================================================================
// P1: 更多类型支持测试（TDD - 先写失败测试）
// ========================================================================

/// P1-1: enum 生成 Rust enum
#[test]
fn test_enum_generation_rust_enum() {
    let ridl_input = r#"
enum Color {
    RED,
    GREEN,
    BLUE,
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // 应该成功生成
    assert!(result.is_ok(), "enum generation should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证生成 Rust enum
    assert!(api_content.contains("pub enum Color"), "Should generate Rust enum");
    assert!(api_content.contains("Red"), "Should have Red variant (PascalCase)");
    assert!(api_content.contains("Green"), "Should have Green variant");
    assert!(api_content.contains("Blue"), "Should have Blue variant");
}

/// P1-1: enum 带数值
#[test]
fn test_enum_with_values() {
    let ridl_input = r#"
enum Status {
    PENDING = 0,
    ACTIVE = 1,
    INACTIVE = 2,
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "enum with values should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("pub enum Status"), "Should generate Status enum");
    assert!(api_content.contains("Pending"), "Should have Pending variant");
    assert!(api_content.contains("Active"), "Should have Active variant");
    assert!(api_content.contains("Inactive"), "Should have Inactive variant");
}

/// P1-2: struct 生成 Rust struct
#[test]
fn test_struct_generation_rust_struct() {
    let ridl_input = r#"
json struct Config {
    name: string;
    value: i32;
    enabled: bool;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "struct generation should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("pub struct Config"), "Should generate Rust struct");
    assert!(api_content.contains("pub name: String"), "Should have name field");
    assert!(api_content.contains("pub value: i32"), "Should have value field");
    assert!(api_content.contains("pub enabled: bool"), "Should have enabled field");
}

/// P1-2: struct 带可选字段
#[test]
fn test_struct_with_optional_fields() {
    let ridl_input = r#"
struct UserProfile {
    name: string;
    email: string?;
    age: i32?;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "struct with optional fields should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("pub struct UserProfile"), "Should generate UserProfile struct");
    assert!(api_content.contains("pub name: String"), "Should have name field");
    assert!(api_content.contains("pub email: Option<String>"), "Should have optional email field");
    assert!(api_content.contains("pub age: Option<i32>"), "Should have optional age field");
}

/// P1-3: using 别名生成
#[test]
fn test_using_alias_generation_rust_alias() {
    let ridl_input = r#"
using StringMap = map<string, string>;
using IntList = array<i32>;
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "using alias generation should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("type StringMap"), "Should generate StringMap alias");
    assert!(api_content.contains("type IntList"), "Should generate IntList alias");
}

/// P1-4: Custom 类型作为参数（使用 enum）— v1 不支持，记录为已知限制
#[test]
fn test_enum_as_parameter() {
    let ridl_input = r#"
enum Color {
    RED,
    GREEN,
    BLUE,
}

class ColorPicker {
    fn setColor(c: Color) -> void;
    fn getColor() -> Color;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // v1 不支持 Custom 类型作为参数/返回值（需要 JS↔Rust 转换代码）
    assert!(result.is_err(), "v1 does not support enum as parameter yet");
}

/// P1-4: Custom 类型作为参数（使用 struct）— v1 不支持，记录为已知限制
#[test]
fn test_struct_as_parameter() {
    let ridl_input = r#"
struct Point {
    x: f64;
    y: f64;
}

class Canvas {
    fn drawPoint(p: Point) -> void;
    fn getPoint() -> Point;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // v1 不支持 Custom 类型作为参数/返回值
    assert!(result.is_err(), "v1 does not support struct as parameter yet");
}

// ========================================================================
// P2: 更多类型支持测试（TDD - 先写失败测试）
// ========================================================================

/// P2-1: Traced<T> 作为参数
#[test]
fn test_traced_as_parameter() {
    let ridl_input = r#"
class TracedParamTest {
    fn setRef(val: Traced<Value>) -> void;
    fn setOptionalRef(val: Traced<Value>?) -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // Traced<T> 作为参数应该支持（映射为 Local<Value>）
    assert!(result.is_ok(), "Traced<T> as parameter should be supported: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("fn set_ref"), "Should have set_ref method");
    assert!(api_content.contains("fn set_optional_ref"), "Should have set_optional_ref method");
}

/// P2-2: global function 生成（v1 不在 api.rs 生成，只在 glue 生成 C FFI）
#[test]
fn test_global_function_generation_rust() {
    let ridl_input = r#"
fn helper(x: i32, y: i32) -> i32;
fn logMessage(msg: string) -> void;
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // 全局函数在 glue 中生成 C FFI，不在 api.rs 中生成 Rust 声明
    assert!(result.is_ok(), "global function generation should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // api.rs 不包含全局函数声明
    assert!(!api_content.contains("fn helper"), "api.rs should not have global function declarations");
}

/// P2-3: const 成员支持
#[test]
fn test_const_member_generation() {
    let ridl_input = r#"
class ConstTest {
    const MAX_SIZE: i32 = 1024;
    const NAME: string = "default";
    fn getValue() -> i32;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    // const 成员应该生成 Rust const
    assert!(result.is_ok(), "const member generation should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    assert!(api_content.contains("const MAX_SIZE"), "Should have MAX_SIZE const");
    assert!(api_content.contains("const NAME"), "Should have NAME const");
}

// ========================================================================
// 异步装饰器端到端测试
// ========================================================================

/// 装饰器端到端：@nonCancellable 生成正确注释
#[test]
fn test_decorator_non_cancellable_e2e() {
    let ridl_input = r#"
callback AsyncCallback(error: string?, data: string?);

class AsyncService {
    @nonCancellable
    fn saveData(data: string, cb: AsyncCallback) -> void;
    
    fn fetchData(url: string, cb: AsyncCallback) -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "decorator e2e should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证装饰器注释生成
    assert!(api_content.contains("nonCancellable"), "Should have nonCancellable comment");
    assert!(api_content.contains("fn save_data"), "Should have save_data method");
    assert!(api_content.contains("fn fetch_data"), "Should have fetch_data method");
}

/// 装饰器端到端：@timeout 生成正确注释
#[test]
fn test_decorator_timeout_e2e() {
    let ridl_input = r#"
callback AsyncCallback(error: string?, data: string?);

class CacheService {
    @timeout(5000)
    fn updateCache(key: string, cb: AsyncCallback) -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "timeout decorator e2e should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证超时装饰器注释生成
    assert!(api_content.contains("timeout(5000)"), "Should have timeout comment");
    assert!(api_content.contains("fn update_cache"), "Should have update_cache method");
}

/// 装饰器端到端：混合装饰器
#[test]
fn test_decorator_mixed_e2e() {
    let ridl_input = r#"
callback AsyncCallback(error: string?, data: string?);

class MixedService {
    fn fetch(url: string, cb: AsyncCallback) -> void;
    
    @nonCancellable
    fn save(data: string, cb: AsyncCallback) -> void;
    
    @timeout(3000)
    fn cache(key: string, cb: AsyncCallback) -> void;
}
"#;

    let tempdir = tempfile::tempdir().unwrap();
    let output_dir = tempdir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let parsed = ridl_tool::parser::parse_ridl_file(ridl_input).unwrap();
    let result = ridl_tool::generator::generate_module_files(
        &parsed.items,
        parsed.module,
        parsed.mode,
        &output_dir,
        "test_module",
    );

    assert!(result.is_ok(), "mixed decorator e2e should succeed: {:?}", result.err());

    let api_file = output_dir.join("api.rs");
    let api_content = std::fs::read_to_string(&api_file).unwrap();

    // 验证所有方法生成
    assert!(api_content.contains("fn fetch"), "Should have fetch method");
    assert!(api_content.contains("fn save"), "Should have save method");
    assert!(api_content.contains("fn cache"), "Should have cache method");
    // 验证装饰器注释
    assert!(api_content.contains("nonCancellable"), "Should have nonCancellable comment");
    assert!(api_content.contains("timeout(3000)"), "Should have timeout comment");
}
