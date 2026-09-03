/// 全面语法覆盖测试
/// 目标：每条语法规则至少 1 个正向 + 1 个反向测试
/// 覆盖：基础语法、复杂语法、各种组合

use ridl_tool::parser::{IDLParser, Rule};
use pest::Parser;

// ========================================================================
// 辅助函数
// ========================================================================

fn parse(rule: Rule, input: &str) -> Result<(), pest::error::Error<Rule>> {
    IDLParser::parse(rule, input).map(|_| ())
}

fn parse_ok(rule: Rule, input: &str, desc: &str) {
    let result = parse(rule, input);
    assert!(result.is_ok(), "{}: 解析失败: {:?}", desc, result.err());
}

fn parse_err(rule: Rule, input: &str, desc: &str) {
    assert!(parse(rule, input).is_err(), "{}: 应该解析失败", desc);
}

// ========================================================================
// 1. 字面量（literal）— 完整覆盖
// ========================================================================

#[test]
fn test_bool_literal_true() {
    parse_ok(Rule::bool_literal, "true", "bool_literal true");
}

#[test]
fn test_bool_literal_false() {
    parse_ok(Rule::bool_literal, "false", "bool_literal false");
}

#[test]
fn test_bool_literal_invalid() {
    parse_err(Rule::bool_literal, "True", "bool_literal 大小写敏感");
    parse_err(Rule::bool_literal, "TRUE", "bool_literal 全大写");
}

#[test]
fn test_null_literal() {
    parse_ok(Rule::null_literal, "null", "null_literal");
}

#[test]
fn test_string_literal_empty() {
    parse_ok(Rule::string_literal, r#""""#, "空字符串");
}

#[test]
fn test_string_literal_with_content() {
    parse_ok(Rule::string_literal, r#""hello world""#, "带内容字符串");
}

#[test]
fn test_string_literal_with_escape() {
    parse_ok(Rule::string_literal, r#""hello\nworld""#, "转义字符");
}

#[test]
fn test_string_literal_unterminated() {
    parse_err(Rule::string_literal, r#""hello"#, "未终止字符串");
}

#[test]
fn test_integer_literal_zero() {
    parse_ok(Rule::integer_literal, "0", "零");
}

#[test]
fn test_integer_literal_positive() {
    parse_ok(Rule::integer_literal, "42", "正整数");
}

#[test]
fn test_integer_literal_negative() {
    // 注意：integer_literal 规则不支持负号，负数由表达式处理
    parse_err(Rule::integer_literal, "-1", "负整数不在 integer_literal 规则中");
}

#[test]
fn test_float_literal_basic() {
    parse_ok(Rule::float_literal, "3.14", "基本浮点数");
}

#[test]
fn test_float_literal_zero() {
    parse_ok(Rule::float_literal, "0.0", "零浮点数");
}

#[test]
fn test_float_literal_no_decimal() {
    parse_err(Rule::float_literal, "3.", "无小数部分");
}

#[test]
fn test_float_literal_no_integer() {
    parse_err(Rule::float_literal, ".14", "无整数部分");
}

// ========================================================================
// 2. 标识符和关键字（identifier, keyword）
// ========================================================================

#[test]
fn test_identifier_basic() {
    parse_ok(Rule::identifier, "foo", "基本标识符");
}

#[test]
fn test_identifier_with_underscore() {
    parse_ok(Rule::identifier, "_foo_bar", "下划线开头");
}

#[test]
fn test_identifier_with_digits() {
    parse_ok(Rule::identifier, "foo123", "包含数字");
}

#[test]
fn test_identifier_start_with_digit() {
    parse_err(Rule::identifier, "123foo", "数字开头");
}

#[test]
fn test_identifier_keyword_conflict() {
    // 所有关键字都不能作为标识符
    let keywords = [
        "interface", "class", "enum", "struct", "const", "readonly",
        "property", "proto", "array", "map", "true", "false", "fn",
        "import", "as", "from", "using", "module", "singleton", "opaque", "Traced",
    ];
    for kw in &keywords {
        parse_err(Rule::identifier, kw, &format!("关键字 '{}' 不能作为标识符", kw));
    }
}

#[test]
fn test_identifier_keyword_prefix() {
    // 以关键字开头但后面跟下划线或字母的应该可以作为标识符
    parse_ok(Rule::identifier, "class_name", "关键字前缀+下划线");
    parse_ok(Rule::identifier, "moduleName", "关键字前缀+字母");
    parse_ok(Rule::identifier, "protoCount", "关键字前缀+驼峰");
    parse_ok(Rule::identifier, "arrayList", "关键字前缀+单词");
    parse_ok(Rule::identifier, "mapData", "map 前缀");
    parse_ok(Rule::identifier, "trueValue", "true 前缀");
}

// ========================================================================
// 3. 注释（comment）
// ========================================================================

#[test]
fn test_single_line_comment() {
    parse_ok(Rule::comment, "// this is a comment\n", "单行注释");
}

#[test]
fn test_multi_line_comment() {
    parse_ok(Rule::comment, "/* multi\nline\ncomment */", "多行注释");
}

#[test]
fn test_nested_comment_content() {
    parse_ok(Rule::comment, "/* comment with * and / chars */", "含特殊字符注释");
}

#[test]
fn test_unclosed_multi_line_comment() {
    parse_err(Rule::comment, "/* unclosed", "未关闭多行注释");
}

// ========================================================================
// 4. 类型系统（type）— 完整覆盖所有变体
// ========================================================================

#[test]
fn test_type_all_basic_types() {
    let types = ["bool", "i32", "i64", "f32", "f64", "string", "void", "object", "null", "any"];
    for t in &types {
        parse_ok(Rule::r#type, t, &format!("基本类型 '{}'", t));
    }
}

#[test]
fn test_type_array() {
    parse_ok(Rule::r#type, "array<i32>", "Array<i32>");
}

#[test]
fn test_type_array_nested() {
    parse_ok(Rule::r#type, "array<array<string>>", "嵌套 Array");
}

#[test]
fn test_type_map() {
    parse_ok(Rule::r#type, "map<string, i32>", "Map<string, i32>");
}

#[test]
fn test_type_map_nested() {
    parse_ok(Rule::r#type, "map<string, map<string, i32>>", "嵌套 Map");
}

#[test]
fn test_type_nullable() {
    parse_ok(Rule::r#type, "string?", "Nullable string");
}

#[test]
fn test_type_nullable_complex() {
    parse_ok(Rule::r#type, "array<i32>?", "Nullable array");
}

#[test]
fn test_type_double_nullable() {
    // parser 实际接受 string??（语法层面不拒绝），语义检查在后续阶段
    parse_ok(Rule::r#type, "string??", "双重 Nullable 语法层面接受");
}

#[test]
fn test_type_union() {
    parse_ok(Rule::r#type, "string | i32", "Union 类型");
}

#[test]
fn test_type_union_multiple() {
    parse_ok(Rule::r#type, "string | i32 | bool", "三元 Union");
}

#[test]
fn test_type_traced() {
    parse_ok(Rule::r#type, "Traced<Value>", "Traced<Value>");
}

#[test]
fn test_type_traced_basic() {
    parse_ok(Rule::r#type, "Traced<i32>", "Traced<i32>");
}

#[test]
fn test_type_traced_nullable() {
    parse_ok(Rule::r#type, "Traced<Value>?", "Nullable Traced");
}

#[test]
fn test_type_traced_in_array() {
    parse_ok(Rule::r#type, "array<Traced<Value>>", "Array<Traced>");
}

#[test]
fn test_type_traced_in_map() {
    parse_ok(Rule::r#type, "map<string, Traced<Value>>", "Map<K, Traced>");
}

#[test]
fn test_type_group() {
    parse_ok(Rule::r#type, "(string)", "Group 类型");
}

#[test]
fn test_type_custom() {
    parse_ok(Rule::r#type, "MyType", "自定义类型");
}

#[test]
fn test_type_custom_with_underscore() {
    parse_ok(Rule::r#type, "my_type", "下划线自定义类型");
}

#[test]
fn test_type_callback() {
    parse_ok(Rule::r#type, "callback(x: i32)", "Callback 类型");
}

#[test]
fn test_type_callback_with_return() {
    // callback_type 规则不包含返回类型，返回类型在 callback_def 中
    parse_ok(Rule::callback_type, "callback(x: i32)", "Callback 类型");
}

// ========================================================================
// 5. 参数（param, param_list）
// ========================================================================

#[test]
fn test_normal_param() {
    parse_ok(Rule::normal_param, "x: i32", "普通参数");
}

#[test]
fn test_normal_param_complex_type() {
    parse_ok(Rule::normal_param, "data: array<string>", "复杂类型参数");
}

#[test]
fn test_variadic_param() {
    parse_ok(Rule::variadic_param, "...args: any", "可变参数");
}

#[test]
fn test_param_list_single() {
    parse_ok(Rule::param_list, "x: i32", "单参数列表");
}

#[test]
fn test_param_list_multiple() {
    parse_ok(Rule::param_list, "x: i32, y: string, z: bool", "多参数列表");
}

#[test]
fn test_param_list_with_variadic() {
    parse_ok(Rule::param_list, "x: i32, ...args: any", "含可变参数");
}

#[test]
fn test_param_list_empty() {
    parse_err(Rule::param_list, "", "空参数列表不应匹配 param_list");
}

// ========================================================================
// 6. 方法定义（method_def）
// ========================================================================

#[test]
fn test_method_no_params_no_return() {
    parse_ok(Rule::method_def, "fn foo()", "无参数无返回值");
}

#[test]
fn test_method_no_params_with_return() {
    parse_ok(Rule::method_def, "fn foo() -> i32", "无参数有返回值");
}

#[test]
fn test_method_with_params() {
    parse_ok(Rule::method_def, "fn foo(x: i32, y: string)", "有参数");
}

#[test]
fn test_method_with_params_and_return() {
    parse_ok(Rule::method_def, "fn foo(x: i32) -> bool", "有参数有返回值");
}

#[test]
fn test_method_missing_fn() {
    parse_err(Rule::method_def, "foo(x: i32)", "缺少 fn 关键字");
}

#[test]
fn test_method_missing_paren() {
    parse_err(Rule::method_def, "fn foo(x: i32", "缺少右括号");
}

// ========================================================================
// 7. 类成员（class_member）— 完整覆盖
// ========================================================================

#[test]
fn test_class_member_const() {
    parse_ok(Rule::const_member, "const MAX: i32 = 100", "const 成员");
}

#[test]
fn test_class_member_var() {
    parse_ok(Rule::var_member, "var count: i32 = 0", "var 成员");
}

#[test]
fn test_class_member_proto_var() {
    parse_ok(Rule::proto_var_member, "proto var x: i32 = 0", "proto var 成员");
}

#[test]
fn test_class_member_proto_readonly_prop() {
    parse_ok(Rule::proto_readonly_prop, "proto readonly property name: string", "proto readonly property");
}

#[test]
fn test_class_member_proto_readwrite_prop() {
    parse_ok(Rule::proto_readwrite_prop, "proto property value: i32", "proto property");
}

#[test]
fn test_class_member_readonly_prop() {
    parse_ok(Rule::readonly_prop, "readonly property name: string", "readonly property");
}

#[test]
fn test_class_member_readwrite_prop() {
    parse_ok(Rule::readwrite_prop, "property value: i32", "readwrite property");
}

#[test]
fn test_class_member_normal_prop() {
    parse_ok(Rule::normal_prop, "name: string", "normal property");
}

#[test]
fn test_class_member_method() {
    parse_ok(Rule::method_def, "fn foo() -> void", "method 成员");
}

#[test]
fn test_class_member_constructor() {
    parse_ok(Rule::class_constructor, "constructor(x: i32)", "constructor 成员");
}

#[test]
fn test_class_member_constructor_compat() {
    parse_ok(Rule::class_constructor_compat, "Foo(x: i32)", "兼容构造函数");
}

// ========================================================================
// 8. 定义（definition）— 完整覆盖所有顶层定义
// ========================================================================

#[test]
fn test_definition_interface() {
    parse_ok(Rule::interface_def, "interface Foo { fn bar() -> void; }", "interface 定义");
}

#[test]
fn test_definition_class() {
    parse_ok(Rule::class_def, "class Foo { fn bar() -> void; }", "class 定义");
}

#[test]
fn test_definition_enum() {
    parse_ok(Rule::enum_def, "enum Foo { A, B, C }", "enum 定义");
}

#[test]
fn test_definition_struct() {
    parse_ok(Rule::struct_def, "struct Foo { x: i32; }", "struct 定义");
}

#[test]
fn test_definition_global_function() {
    parse_ok(Rule::global_function, "fn foo(x: i32) -> void;", "全局函数");
}

#[test]
fn test_definition_callback() {
    parse_ok(Rule::callback_def, "callback Foo(x: i32);", "callback 定义");
}

#[test]
fn test_definition_using() {
    parse_ok(Rule::using_def, "using Foo = i32;", "using 定义");
}

#[test]
fn test_definition_import() {
    parse_ok(Rule::import_stmt, "import { Foo } from \"bar\";", "import 定义");
}

#[test]
fn test_definition_singleton() {
    parse_ok(Rule::singleton_def, "singleton Foo { fn bar() -> void; }", "singleton 定义");
}

// ========================================================================
// 9. 模块声明（module_decl）— 完整覆盖
// ========================================================================

#[test]
fn test_module_simple() {
    parse_ok(Rule::module_decl, "module foo@1.0;", "简单模块");
}

#[test]
fn test_module_with_version() {
    parse_ok(Rule::module_decl, "module foo@1.0;", "带版本模块");
}

#[test]
fn test_module_with_path() {
    parse_ok(Rule::module_decl, "module foo.bar@1.0;", "带路径模块");
}

#[test]
fn test_module_three_part_version() {
    parse_ok(Rule::module_decl, "module foo@1.2.3;", "三段版本");
}

#[test]
fn test_module_no_semicolon() {
    parse_ok(Rule::module_decl, "module foo@1.0", "无分号");
}

#[test]
fn test_module_spaces_around_at() {
    parse_err(Rule::module_decl, "module foo @ 1.0;", "@ 周围有空格");
}

// ========================================================================
// 10. 模式声明（mode_decl）
// ========================================================================

#[test]
fn test_mode_strict() {
    parse_ok(Rule::mode_decl, "mode strict;", "strict 模式");
}

#[test]
fn test_mode_custom() {
    parse_ok(Rule::mode_decl, "mode my_mode;", "自定义模式");
}

#[test]
fn test_mode_missing_semicolon() {
    parse_err(Rule::mode_decl, "mode strict", "缺少分号");
}

// ========================================================================
// 11. Opaque 块 — 完整覆盖
// ========================================================================

#[test]
fn test_opaque_empty() {
    parse_ok(Rule::opaque_block, "opaque { }", "空 opaque 块");
}

#[test]
fn test_opaque_single_field() {
    parse_ok(Rule::opaque_block, "opaque { x: i32 }", "单字段");
}

#[test]
fn test_opaque_multiple_fields() {
    parse_ok(Rule::opaque_block, "opaque { x: i32; y: string; z: bool }", "多字段");
}

#[test]
fn test_opaque_traced_field() {
    parse_ok(Rule::opaque_block, "opaque { held: Traced<Value> }", "Traced 字段");
}

#[test]
fn test_opaque_optional_traced() {
    parse_ok(Rule::opaque_block, "opaque { held: Traced<Value>? }", "Optional Traced");
}

#[test]
fn test_opaque_array_traced() {
    parse_ok(Rule::opaque_block, "opaque { items: array<Traced<Value>> }", "Array Traced");
}

#[test]
fn test_opaque_map_traced() {
    parse_ok(Rule::opaque_block, "opaque { cache: map<string, Traced<Value>> }", "Map Traced");
}

#[test]
fn test_opaque_mixed_types() {
    parse_ok(Rule::opaque_block, "opaque { held: Traced<Value>; count: i32; name: string }", "混合类型");
}

#[test]
fn test_opaque_comma_separated() {
    parse_ok(Rule::opaque_block, "opaque { x: i32, y: string }", "逗号分隔");
}

#[test]
fn test_opaque_semicolon_separated() {
    parse_ok(Rule::opaque_block, "opaque { x: i32; y: string }", "分号分隔");
}

#[test]
fn test_opaque_missing_brace() {
    parse_err(Rule::opaque_block, "opaque { x: i32", "缺少右花括号");
}

#[test]
fn test_opaque_missing_type() {
    parse_err(Rule::opaque_block, "opaque { x: }", "缺少类型");
}

// ========================================================================
// 12. 枚举（enum_def, enum_value）
// ========================================================================

#[test]
fn test_enum_simple() {
    parse_ok(Rule::enum_def, "enum Color { RED, GREEN, BLUE }", "简单枚举");
}

#[test]
fn test_enum_with_values() {
    parse_ok(Rule::enum_def, "enum Color { RED = 0, GREEN = 1, BLUE = 2 }", "带值枚举");
}

#[test]
fn test_enum_trailing_comma() {
    parse_ok(Rule::enum_def, "enum Color { RED, GREEN, BLUE, }", "尾逗号");
}

#[test]
fn test_enum_single_value() {
    parse_ok(Rule::enum_def, "enum Color { RED }", "单值枚举");
}

#[test]
fn test_enum_empty() {
    parse_err(Rule::enum_def, "enum Color { }", "空枚举");
}

#[test]
fn test_enum_value_simple() {
    parse_ok(Rule::enum_value, "RED", "简单枚举值");
}

#[test]
fn test_enum_value_with_number() {
    parse_ok(Rule::enum_value, "RED = 0", "带数字枚举值");
}

// ========================================================================
// 13. 结构体（struct_def）
// ========================================================================

#[test]
fn test_struct_simple() {
    parse_ok(Rule::struct_def, "struct Point { x: i32; y: i32; }", "简单结构体");
}

#[test]
fn test_struct_json() {
    parse_ok(Rule::struct_def, "json struct Config { name: string; }", "JSON 结构体");
}

#[test]
fn test_struct_msgpack() {
    parse_ok(Rule::struct_def, "msgpack struct Data { value: i32; }", "MsgPack 结构体");
}

#[test]
fn test_struct_protobuf() {
    parse_ok(Rule::struct_def, "protobuf struct Message { id: i32; }", "Protobuf 结构体");
}

#[test]
fn test_struct_empty() {
    // parser 实际接受空结构体（语法层面允许），语义检查在后续阶段
    parse_ok(Rule::struct_def, "struct Empty { }", "空结构体语法层面接受");
}

// ========================================================================
// 14. 导入（import_stmt）— TypeScript 风格花括号语法
// ========================================================================

#[test]
fn test_import_single() {
    parse_ok(Rule::import_stmt, r#"import { Foo } from "bar";"#, "单个导入");
}

#[test]
fn test_import_multiple() {
    parse_ok(Rule::import_stmt, r#"import { Foo, Bar, Baz } from "lib";"#, "多个导入");
}

#[test]
fn test_import_alias() {
    parse_ok(Rule::import_stmt, r#"import { Foo as F } from "bar";"#, "别名导入");
}

#[test]
fn test_import_mixed_alias() {
    parse_ok(Rule::import_stmt, r#"import { Foo, Bar as B } from "lib";"#, "混合别名");
}

#[test]
fn test_import_trailing_comma() {
    parse_ok(Rule::import_stmt, r#"import { Foo, Bar, } from "lib";"#, "尾逗号");
}

#[test]
fn test_import_wildcard() {
    parse_ok(Rule::import_stmt, r#"import * as Bar from "baz";"#, "通配符导入");
}

#[test]
fn test_import_missing_from() {
    parse_err(Rule::import_stmt, r#"import { Foo } "bar";"#, "缺少 from");
}

#[test]
fn test_import_missing_semicolon() {
    parse_err(Rule::import_stmt, r#"import { Foo } from "bar""#, "缺少分号");
}

#[test]
fn test_import_missing_brace() {
    parse_err(Rule::import_stmt, r#"import { Foo from "bar";"#, "花括号未关闭");
}

#[test]
fn test_import_old_syntax_rejected() {
    parse_err(Rule::import_stmt, r#"import Foo from "bar";"#, "旧语法已废弃");
}

// ========================================================================
// 15. 单例（singleton_def, singleton_member）
// ========================================================================

#[test]
fn test_singleton_with_method() {
    parse_ok(Rule::singleton_def, "singleton Foo { fn bar() -> void; }", "单例方法");
}

#[test]
fn test_singleton_with_readonly_prop() {
    parse_ok(Rule::singleton_def, "singleton Foo { readonly property name: string; }", "单例只读属性");
}

#[test]
fn test_singleton_with_readwrite_prop() {
    parse_ok(Rule::singleton_def, "singleton Foo { property value: i32; }", "单例读写属性");
}

#[test]
fn test_singleton_multiple_members() {
    parse_ok(Rule::singleton_def, "singleton Foo { fn bar() -> void; readonly property x: i32; property y: string; }", "多成员单例");
}

#[test]
fn test_singleton_empty() {
    // parser 实际接受空单例（语法层面允许），语义检查在后续阶段
    parse_ok(Rule::singleton_def, "singleton Foo { }", "空单例语法层面接受");
}

// ========================================================================
// 16. 回调（callback_def, callback_type）
// ========================================================================

#[test]
fn test_callback_no_params() {
    parse_ok(Rule::callback_def, "callback Foo();", "无参回调");
}

#[test]
fn test_callback_with_params() {
    parse_ok(Rule::callback_def, "callback Foo(x: i32, y: string);", "有参回调");
}

#[test]
fn test_callback_type_no_params() {
    parse_ok(Rule::callback_type, "callback()", "无参回调类型");
}

#[test]
fn test_callback_type_with_params() {
    parse_ok(Rule::callback_type, "callback(x: i32)", "有参回调类型");
}

#[test]
fn test_callback_type_named() {
    parse_ok(Rule::callback_type, "callback MyCallback(x: i32)", "命名回调类型");
}

// ========================================================================
// 17. 接口（interface_def）
// ========================================================================

#[test]
fn test_interface_empty() {
    parse_ok(Rule::interface_def, "interface Foo { }", "空接口");
}

#[test]
fn test_interface_single_method() {
    parse_ok(Rule::interface_def, "interface Foo { fn bar() -> void; }", "单方法接口");
}

#[test]
fn test_interface_multiple_methods() {
    parse_ok(Rule::interface_def, "interface Foo { fn bar() -> void; fn baz(x: i32) -> bool; }", "多方法接口");
}

#[test]
fn test_interface_missing_brace() {
    parse_err(Rule::interface_def, "interface Foo fn bar() -> void; }", "缺少左花括号");
}

// ========================================================================
// 18. Using 定义
// ========================================================================

#[test]
fn test_using_basic() {
    parse_ok(Rule::using_def, "using MyInt = i32;", "基本 using");
}

#[test]
fn test_using_complex_type() {
    parse_ok(Rule::using_def, "using StringMap = map<string, string>;", "复杂类型 using");
}

#[test]
fn test_using_missing_semicolon() {
    parse_err(Rule::using_def, "using MyInt = i32", "缺少分号");
}

// ========================================================================
// 19. 综合测试 — 复杂组合
// ========================================================================

#[test]
fn test_full_idl_with_all_features() {
    let input = r#"
mode strict;
module my.lib@1.0.0;

import { Helper } from "helper_lib";

using StringMap = map<string, string>;

callback EventHandler(event: object);

interface Drawable {
    fn draw(ctx: object) -> void;
    fn getBounds() -> array<f32>;
}

enum Color {
    RED = 0,
    GREEN = 1,
    BLUE = 2,
}

json struct Config {
    name: string;
    value: i32;
}

singleton AppService {
    fn initialize() -> void;
    readonly property version: string;
    property debug: bool;
}

class Widget {
    const MAX_SIZE: i32 = 1024;
    var count: i32 = 0;
    proto var protoState: i32 = 0;
    proto readonly property protoId: string;
    proto property protoValue: i32;
    readonly property id: string;
    property name: string;
    constructor(x: i32, y: i32);
    fn render() -> void;
    fn getPosition() -> array<f32>;
    opaque {
        held: Traced<Value>
        items: array<Traced<Value>>
        cache: map<string, Traced<Value>>
        optional_data: Traced<Value>?
        plain_count: i32
        label: string
    }
}
"#;
    parse_ok(Rule::idl, input, "完整 IDL 文件");
}

#[test]
fn test_class_with_all_member_types() {
    let input = r#"
class FullClass {
    const MAX: i32 = 100;
    var count: i32 = 0;
    proto var protoCount: i32 = 0;
    proto readonly property protoName: string;
    proto property protoValue: i32;
    readonly property name: string;
    property value: i32;
    constructor(x: i32);
    fn doSomething(a: string, b: i32) -> bool;
    opaque {
        held: Traced<Value>
        data: array<Traced<Value>>
    }
}
"#;
    parse_ok(Rule::class_def, input, "所有成员类型的 class");
}

#[test]
fn test_nested_complex_types() {
    parse_ok(Rule::r#type, "map<string, array<Traced<Value>?>>", "嵌套 Map<Array<Optional<Traced>>>");
    parse_ok(Rule::r#type, "array<map<string, Traced<Value>>>", "嵌套 Array<Map<K, Traced>>");
    parse_ok(Rule::r#type, "(string | i32)?", "Nullable Group Union");
    parse_ok(Rule::r#type, "array<string | i32>", "Array<Union>");
}

#[test]
fn test_error_recovery_missing_semicolon() {
    // 缺少分号应该报错
    parse_err(Rule::class_def, "class Foo { fn bar() -> void }", "class 方法缺少分号");
    parse_err(Rule::interface_def, "interface Foo { fn bar() -> void }", "interface 方法缺少分号");
}

#[test]
fn test_error_recovery_extra_semicolon() {
    // 多余分号应该报错
    parse_err(Rule::class_def, "class Foo { fn bar() -> void;; }", "多余分号");
}

#[test]
fn test_whitespace_handling() {
    // 各种空白应该被正确处理
    parse_ok(Rule::class_def, "class Foo{fn bar()->void;}", "无空白");
    parse_ok(Rule::class_def, "class  Foo  {  fn  bar()  ->  void;  }", "多余空白");
    parse_ok(Rule::class_def, "class\tFoo\t{\tfn\tbar()\t->\tvoid;\t}", "制表符");
    parse_ok(Rule::class_def, "class\nFoo\n{\nfn\nbar()\n->\nvoid;\n}", "换行符");
}
