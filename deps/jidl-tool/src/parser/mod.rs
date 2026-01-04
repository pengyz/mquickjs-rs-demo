use pest_derive::Parser;
use pest::Parser;
use crate::parser::ast::Param;
use crate::parser::ast::SerializationFormat;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct IDLParser;

pub mod ast;

use ast::{IDLItem, Interface, Class, Enum, Function, Type, Field, Property, Method, StructDef, PropertyModifier};

/// 解析IDL内容
pub fn parse_idl(content: &str) -> Result<Vec<IDLItem>, Box<dyn std::error::Error>> {
    let mut pairs = IDLParser::parse(Rule::idl, content)
        .map_err(|e| format!("Parse error: {}", e))?;

    // 获取idl规则内部的定义
    let idl_pair = pairs.next().unwrap();
    let mut items = Vec::new();
    
    // 遍历idl内部的元素（定义、WS等）
    for pair in idl_pair.into_inner() {
        match pair.as_rule() {
            Rule::interface_def => {
                items.push(parse_interface(pair)?);
            }
            Rule::class_def => {
                items.push(parse_class(pair)?);
            }
            Rule::enum_def => {
                items.push(parse_enum(pair)?);
            }
            Rule::struct_def => {
                items.push(parse_struct(pair)?);
            }
            Rule::global_function => {
                items.push(parse_global_function(pair)?);
            }
            Rule::WS => {} // 空白符，跳过
            Rule::EOI => {} // 文件结束符，跳过
            _ => {} // 其他规则，跳过
        }
    }

    Ok(items)
}

fn parse_interface(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut interface_pairs = pair.into_inner();
    let mut interface_name: Option<String> = None;
    let mut methods = Vec::new();
    
    for inner_pair in interface_pairs {
        match inner_pair.as_rule() {
            Rule::identifier => {
                if interface_name.is_none() {
                    interface_name = Some(inner_pair.as_str().to_string());
                }
            }
            Rule::interface_member => {
                methods.push(parse_interface_method(inner_pair)?);
            }
            Rule::WS => {} // 跳过空白
            Rule::EOI => {} // 跳过结束符（理论上不应该在这里出现）
            _ => {} // 其他情况跳过
        }
    }
    
    let interface_name = interface_name.ok_or("Missing interface name")?;
    
    Ok(IDLItem::Interface(Interface {
        name: interface_name,
        methods,
        properties: Vec::new(), // 接口通常不包含属性
    }))
}

fn parse_interface_method(pair: pest::iterators::Pair<Rule>) -> Result<Method, Box<dyn std::error::Error>> {
    let mut pairs = pair.into_inner();
    
    let name = pairs.next().ok_or("Expected method name")?.as_str().to_string();
    
    let params_list = pairs.next().ok_or("Expected method parameters")?;
    let mut params = Vec::new();
    
    if params_list.as_rule() == Rule::param_list {
        for param_pair in params_list.into_inner() {
            if param_pair.as_rule() == Rule::param {
                let mut param_inner = param_pair.into_inner();
                let param_name = param_inner.next().ok_or("Expected parameter name")?.as_str();
                let type_pair = param_inner.next().ok_or("Expected parameter type")?;
                let param_type = parse_type(type_pair)?;
                params.push(Param {
                    name: param_name.to_string(),
                    param_type: param_type,
                    optional: false, // Parameters are not optional by default
                });
            }
        }
    }
    
    // Check if there's a return type
    let return_type = if let Some(return_type_pair) = pairs.next() {
        if return_type_pair.as_rule() == Rule::r#type {
            parse_type(return_type_pair)?
        } else {
            Type::Void
        }
    } else {
        Type::Void
    };
    
    Ok(Method {
        name,
        params,
        return_type,
        is_async: false,
    })
}

fn parse_class(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut pairs = pair.into_inner();

    let class_name = pairs
        .next()
        .ok_or("Expected class name")?
        .as_str()
        .to_string();

    let mut inner_pairs = pairs.next().ok_or("Expected class body")?.into_inner();

    let mut constructor = None;
    let mut properties = Vec::new();
    let mut methods = Vec::new();

    for inner_pair in inner_pairs {
        match inner_pair.as_rule() {
            Rule::identifier => {
                // This could be a constructor if it matches the class name
                let name = inner_pair.as_str();
                if name == class_name {
                    // Parse constructor
                    let constructor_content = inner_pair.clone().into_inner().next().ok_or("Constructor needs parameters")?;
                    let mut param_pairs = constructor_content.into_inner();
                    
                    let mut params = Vec::new();
                    if let Some(params_list) = param_pairs.next() {
                        if params_list.as_rule() == Rule::param_list {
                            for param_pair in params_list.into_inner() {
                                if param_pair.as_rule() == Rule::param {
                                    let mut param_inner = param_pair.into_inner();
                                    let param_name = param_inner.next().ok_or("Expected parameter name")?.as_str();
                                    let type_pair = param_inner.next().ok_or("Expected parameter type")?;
                                    let param_type = parse_type(type_pair)?;
                                    params.push(Param {
                                        name: param_name.to_string(),
                                        param_type: param_type,
                                        optional: false, // Parameters are not optional by default
                                    });
                                }
                            }
                        }
                    }
                    
                    constructor = Some(Function {
                        name: name.to_string(),
                        return_type: Type::Void, // Constructor returns void
                        params,
                        is_async: false,
                    });
                }
            }
            Rule::const_member => {
                // Parse constant member
                let mut const_pairs = inner_pair.into_inner();
                let name = const_pairs.next().ok_or("Expected const name")?.as_str().to_string();
                let type_content = const_pairs.next().ok_or("Expected const type")?;
                let prop_type = parse_type(type_content)?;
                let value = const_pairs.next().ok_or("Expected const value")?.as_str().to_string();
                
                properties.push(Property {
                    modifiers: vec![PropertyModifier::Const],
                    name,
                    property_type: prop_type,
                    default_value: Some(value),
                });
            }
            Rule::readonly_prop => {
                // Parse readonly property
                let mut prop_pairs = inner_pair.into_inner();
                let name = prop_pairs.next().ok_or("Expected readonly property name")?.as_str().to_string();
                let type_content = prop_pairs.next().ok_or("Expected readonly property type")?;
                let prop_type = parse_type(type_content)?;
                
                properties.push(Property {
                    modifiers: vec![PropertyModifier::Readonly],
                    name,
                    property_type: prop_type,
                    default_value: None,
                });
            }
            Rule::readwrite_prop => {
                // Parse readwrite property
                let mut prop_pairs = inner_pair.into_inner();
                let name = prop_pairs.next().ok_or("Expected readwrite property name")?.as_str().to_string();
                let type_content = prop_pairs.next().ok_or("Expected readwrite property type")?;
                let prop_type = parse_type(type_content)?;
                
                properties.push(Property {
                    modifiers: vec![PropertyModifier::ReadWrite],
                    name,
                    property_type: prop_type,
                    default_value: None,
                });
            }
            Rule::class_method => {
                // Parse class method
                let mut method_pairs = inner_pair.into_inner();
                let name = method_pairs.next().ok_or("Expected method name")?.as_str().to_string();
                
                let params_list = method_pairs.next().ok_or("Expected method parameters")?;
                let mut params = Vec::new();
                
                if params_list.as_rule() == Rule::param_list {
                    for param_pair in params_list.into_inner() {
                        if param_pair.as_rule() == Rule::param {
                            let mut param_inner = param_pair.into_inner();
                            let param_name = param_inner.next().ok_or("Expected parameter name")?.as_str();
                            let type_pair = param_inner.next().ok_or("Expected parameter type")?;
                            let param_type = parse_type(type_pair)?;
                            params.push(Param {
                                name: param_name.to_string(),
                                param_type: param_type,
                                optional: false, // Parameters are not optional by default
                            });
                        }
                    }
                }
                
                // Check if there's a return type
                let return_type = if let Some(return_type_pair) = method_pairs.next() {
                    if return_type_pair.as_rule() == Rule::r#type {
                        parse_type(return_type_pair)?
                    } else {
                        Type::Void
                    }
                } else {
                    Type::Void
                };
                
                methods.push(Method {
                    name,
                    params,
                    return_type,
                    is_async: false,
                });
            }
            Rule::constructor => {
                // Parse constructor
                let mut constructor_pairs = inner_pair.into_inner();
                let name = constructor_pairs.next().ok_or("Expected constructor name")?.as_str().to_string();
                
                let params_list = constructor_pairs.next().ok_or("Expected constructor parameters")?;
                let mut params = Vec::new();
                
                if params_list.as_rule() == Rule::param_list {
                    for param_pair in params_list.into_inner() {
                        if param_pair.as_rule() == Rule::param {
                            let mut param_inner = param_pair.into_inner();
                            let param_name = param_inner.next().ok_or("Expected parameter name")?.as_str();
                            let type_pair = param_inner.next().ok_or("Expected parameter type")?;
                            let param_type = parse_type(type_pair)?;
                            params.push(Param {
                                name: param_name.to_string(),
                                param_type: param_type,
                                optional: false, // Parameters are not optional by default
                            });
                        }
                    }
                }
                
                constructor = Some(Function {
                    name,
                    return_type: Type::Void, // Constructor returns void
                    params,
                    is_async: false,
                });
            }
            Rule::WS => {} // Skip whitespace
            Rule::EOI => {} // Skip end of input
            _ => {
                eprintln!("Unexpected rule in class: {:?}", inner_pair.as_rule());
            }
        }
    }

    Ok(IDLItem::Class(Class {
        name: class_name,
        constructor,
        properties,
        methods,
    }))
}

fn parse_enum(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut enum_pairs = pair.into_inner();
    
    // 获取枚举名称
    let enum_name_pair = enum_pairs.next().ok_or("Missing enum name")?;
    
    // 解析枚举值
    let mut values = Vec::new();
    for inner_pair in enum_pairs {
        match inner_pair.as_rule() {
            Rule::enum_value => {
                let mut value_pairs = inner_pair.into_inner();
                let value_name_pair = value_pairs.next().ok_or("Missing enum value name")?;
                
                // 检查是否有显式的值
                let mut explicit_value = None;
                if let Some(value) = value_pairs.next() {
                    if value.as_rule() == Rule::number_literal {
                        explicit_value = Some(value.as_str().parse::<i32>()?);
                    }
                }
                
                values.push(ast::EnumValue {
                    name: value_name_pair.as_str().to_string(),
                    value: explicit_value,
                });
            }
            Rule::WS => {} // 跳过空白
            _ => {} // 其他规则忽略
        }
    }
    
    Ok(IDLItem::Enum(ast::Enum {
        name: enum_name_pair.as_str().to_string(),
        values,
    }))
}

fn parse_struct(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // 检查是否有序列化格式声明
    let mut serialization_format = SerializationFormat::Json; // 默认为JSON
    
    // 获取第一个元素，检查是否是格式声明
    let mut current_pair = inner_pairs.next().unwrap();
    
    // 检查第一个元素是否是格式声明
    if matches!(current_pair.as_rule(), Rule::json_format | Rule::msgpack_format | Rule::protobuf_format) {
        match current_pair.as_rule() {
            Rule::json_format => {
                serialization_format = SerializationFormat::Json;
            }
            Rule::msgpack_format => {
                serialization_format = SerializationFormat::MsgPack;
            }
            Rule::protobuf_format => {
                serialization_format = SerializationFormat::Protobuf;
            }
            _ => {}
        }
        // 如果有格式声明，下一个应该是"struct"关键字
        current_pair = inner_pairs.next().unwrap();
    } else {
        // 如果没有格式声明，当前元素应该是"struct"关键字
        // 不需要改变 current_pair
    }
    
    // 现在 current_pair 应该是"struct"关键字（作为字符串字面量），跳过它
    // 字符串字面量（如"struct"）在解析树中不会生成独立的规则，而是嵌入到结构中
    // 所以现在应该是标识符（结构体名称）
    if current_pair.as_str() == "struct" {
        // 移动到下一个，应该是结构体名称
        current_pair = inner_pairs.next().unwrap();
    }
    
    // 现在 current_pair 应该是结构体名称（identifier）
    let name = current_pair.as_str().to_string();
    
    // 现在处理字段定义
    let mut fields = Vec::new();
    for pair in inner_pairs {
        match pair.as_rule() {
            Rule::field_def => {
                fields.push(parse_field(pair)?);
            }
            Rule::WS => {
                // 跳过空白
            }
            Rule::identifier => {
                // 这是结构体名称，已经处理过了，跳过
            }
            _ => {
                // 检查是否是"struct"字符串字面量，跳过它
                if pair.as_str() != "struct" && pair.as_rule() != Rule::EOI {
                    eprintln!("Unexpected rule in struct: {:?} -> '{}'", pair.as_rule(), pair.as_str());
                }
            }
        }
    }
    
    Ok(IDLItem::Struct(StructDef {
        name,
        fields,
        serialization_format,
    }))
}

fn parse_field(pair: pest::iterators::Pair<Rule>) -> Result<Field, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // identifier
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();
    
    // ":" type
    let type_pair = inner_pairs.next().unwrap();
    let field_type = parse_type(type_pair)?;
    
    Ok(Field {
        name,
        field_type,
        optional: false, // 简化处理，不支持可选字段
    })
}

fn parse_function(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut func_pairs = pair.into_inner();
    
    // 跳过"fn"关键字
    let _fn_keyword = func_pairs.next().ok_or("Missing 'fn' keyword")?;
    
    // 解析函数名称
    let func_name_pair = func_pairs.next().ok_or("Missing function name")?;
    
    // 解析参数列表和返回类型
    let mut params = Vec::new();
    let mut return_type = None;
    
    for inner_pair in func_pairs {
        match inner_pair.as_rule() {
            Rule::param => {
                let mut param_pairs = inner_pair.into_inner();
                let param_name_pair = param_pairs.next().ok_or("Missing parameter name")?;
                let param_type_pair = param_pairs.next().ok_or("Missing parameter type")?;
                
                params.push(ast::Param {
                    name: param_name_pair.as_str().to_string(),
                    param_type: parse_type(param_type_pair)?,
                    optional: false, // 默认为非可选
                });
            }
            Rule::r#type => {
                // 这是返回类型
                return_type = Some(parse_type(inner_pair)?);
            }
            Rule::WS => {} // 跳过空白
            _ => {} // 其他规则忽略
        }
    }
    
    let return_type = return_type.ok_or("Missing function return type")?;
    
    Ok(IDLItem::GlobalFunction(ast::Function {
        name: func_name_pair.as_str().to_string(),
        return_type,
        params,
        is_async: false, // 默认为同步函数
    }))
}

fn parse_global_function(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    // 调用现有的 parse_function 函数来解析全局函数
    parse_function(pair)
}

fn parse_method(pair: pest::iterators::Pair<Rule>) -> Result<Method, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // 跳过 "fn" 关键字
    let mut pair_iter = inner_pairs.peekable();
    let fn_keyword = pair_iter.next();
    if fn_keyword.map(|p| p.as_str()) != Some("fn") {
        return Err("Expected 'fn' keyword".into());
    }
    
    // 方法名
    let name_pair = pair_iter.next().ok_or("Expected method name")?;
    if name_pair.as_rule() != Rule::identifier {
        return Err("Method name must be an identifier".into());
    }
    let name = name_pair.as_str().to_string();
    
    // 参数列表
    let mut params = Vec::new();
    let mut return_type = Type::Void;
    let mut has_return_type = false;
    
    for p in pair_iter {
        match p.as_rule() {
            Rule::param_list => {
                params = parse_param_list(p)?;
            }
            Rule::r#type => {
                return_type = parse_type(p)?;
                has_return_type = true;
            }
            Rule::WS => {
                // 忽略空白
            }
            _ => {
                // 其他元素，目前忽略
            }
        }
    }
    
    Ok(Method {
        name,
        params,
        return_type: if has_return_type { return_type } else { Type::Void },
        is_async: false,
    })
}

fn parse_param_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Param>, Box<dyn std::error::Error>> {
    let mut params = Vec::new();
    let mut inner_pairs = pair.into_inner();
    
    // 第一个参数
    if let Some(first_param) = inner_pairs.next() {
        params.push(parse_param(first_param)?);
    }
    
    // 其余参数
    for pair in inner_pairs {
        if pair.as_rule() == Rule::param {
            params.push(parse_param(pair)?);
        }
    }
    
    Ok(params)
}

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Result<ast::Param, Box<dyn std::error::Error>> {
    let mut param_pairs = pair.into_inner();
    
    let param_type_pair = param_pairs.next().ok_or("Missing param type")?; // type
    let param_name_pair = param_pairs.next().ok_or("Missing param name")?; // identifier
    
    Ok(ast::Param {
        name: param_name_pair.as_str().to_string(),
        param_type: parse_type(param_type_pair)?,
        optional: false, // 暂时默认为非可选
    })
}

fn parse_type(pair: pest::iterators::Pair<Rule>) -> Result<Type, Box<dyn std::error::Error>> {
    // 简单实现，根据类型名称映射到Type枚举
    let type_str = pair.as_str();
    let result = match type_str {
        "bool" => Type::Bool,
        "int" => Type::Int,
        "float" => Type::Float,
        "string" => Type::String,
        "void" => Type::Void,
        "any" => Type::Any,
        _ => Type::Custom(type_str.to_string()),
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use pest::Parser;
    use crate::parser::{IDLParser, Rule};
    
    #[test]
    fn test_identifier_parsing() {
        let result = IDLParser::parse(Rule::identifier, "TestInterface");
        assert!(result.is_ok());
    }

    #[test]
    fn test_interface_parsing() {
        let result = IDLParser::parse(Rule::interface_def, "interface TestInterface { doSomething(value: int) -> void; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_type_parsing() {
        let result = IDLParser::parse(Rule::basic_type, "int");
        assert!(result.is_ok());
    }

    #[test]
    fn test_class_parsing() {
        let result = IDLParser::parse(Rule::class_def, "class TestClass { const MAX_SIZE: int = 100; property value: string; TestClass(initialValue: string); methodA(param1: int) -> string; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_interface_parsing() {
        let result = IDLParser::parse(Rule::interface_def, "interface SimpleInterface { getValue() -> int; setValue(value: int); }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_format_parsing() {
        let result = IDLParser::parse(Rule::json_format, "json");
        assert!(result.is_ok());
    }

    #[test]
    fn test_msgpack_format_parsing() {
        let result = IDLParser::parse(Rule::msgpack_format, "msgpack");
        assert!(result.is_ok());
    }

    #[test]
    fn test_protobuf_format_parsing() {
        let result = IDLParser::parse(Rule::protobuf_format, "protobuf");
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_struct_parsing() {
        let input = "json struct TestStruct { name: string; count: int; active: bool; }";
        let result = IDLParser::parse(Rule::struct_def, input);
        assert!(result.is_ok(), "Failed to parse JSON struct definition");
        
        // 进一步验证解析结果是否符合预期
        let mut pairs = result.unwrap();
        let struct_pair = pairs.next().unwrap();
        let inner_rules: Vec<_> = struct_pair.into_inner().map(|p| (p.as_rule(), p.as_str())).collect();
        
        // 验证规则序列: json_format -> identifier -> field_def (x3)
        // 注意："struct" 作为字符串字面量不产生独立的解析树节点
        assert_eq!(inner_rules.len(), 5);
        assert_eq!(inner_rules[0].0, Rule::json_format);
        assert_eq!(inner_rules[0].1, "json");
        assert_eq!(inner_rules[1].0, Rule::identifier);
        assert_eq!(inner_rules[1].1, "TestStruct");
        assert_eq!(inner_rules[2].0, Rule::field_def);
        assert_eq!(inner_rules[3].0, Rule::field_def);
        assert_eq!(inner_rules[4].0, Rule::field_def);
    }
    
    #[test]
    fn test_complex_ridl_example() {
        use crate::parser::{IDLParser, Rule};
        
        let ridl_content = r#"
// 日志级别枚举
import NetworkPacket from "Packet.proto";

enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3
}

// 回调函数类型定义
callback ProcessCallback(result: string | object, success: bool);

// 日志条目结构体
json struct LogEntry {
    level: LogLevel;
    message: string;
    timestamp: int;
    metadata: map<string, string>;
}

// 网络日志处理器接口
interface NetworkLogHandler {
    sendLog(entry: LogEntry) -> bool;
    sendLogBatch(entries: array<LogEntry>) -> int;
    setLogLevel(level: LogLevel);
    getLogLevel() -> LogLevel;
}

// 日志处理器实现类
class LogHandler {
    const MAX_LOG_SIZE: int = 1024;        // 最大日志大小常量
    readonly property currentLogLevel: LogLevel;  // 只读属性
    property logBuffer: array<LogEntry>;         // 读写属性

    // 构造函数
    LogHandler(initialLevel: LogLevel);
    
    // 方法定义
    processLog(message: string, level: LogLevel) -> bool;
    flushLogs() -> array<LogEntry>;
}
"#;

        let result = IDLParser::parse(Rule::idl, ridl_content);
        match result {
            Ok(_) => {
                println!("Complex RIDL example parsed successfully");
            },
            Err(e) => {
                println!("Complex RIDL Error: {}", e);
                panic!("Failed to parse complex RIDL example");
            }
        }
    }
    
    #[test]
    fn test_full_ridl_example_from_spec() {
        use crate::parser::{IDLParser, Rule};
        
        let ridl_content = r#"
// 日志级别枚举
import NetworkPacket from "Packet.proto";

enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3
}

// 回调函数类型定义
callback ProcessCallback(result: string | object, success: bool);  // 回调函数无返回值

// 日志条目结构体
json struct LogEntry {
    level: LogLevel;
    message: string;
    timestamp: int;
    metadata: map<string, string>;
}

// 网络日志处理器接口
interface NetworkLogHandler {
    sendLog(entry: LogEntry) -> bool;
    sendLogBatch(entries: array<LogEntry>) -> int;
    setLogLevel(level: LogLevel);
    getLogLevel() -> LogLevel;
}

// 日志处理器实现类
class LogHandler {
    const MAX_LOG_SIZE: int = 1024;        // 最大日志大小常量
    readonly property currentLogLevel: LogLevel;  // 只读属性
    property logBuffer: array<LogEntry>;         // 读写属性

    // 构造函数
    LogHandler(initialLevel: LogLevel);
    
    // 方法定义
    processLog(message: string, level: LogLevel) -> bool;
    flushLogs() -> array<LogEntry>;
}
"#;

        let result = IDLParser::parse(Rule::idl, ridl_content);
        match result {
            Ok(pairs) => {
                println!("Full RIDL example from spec parsed successfully");
                // 验证解析出的项目数量
                let count = pairs.clone().flatten().count();
                assert!(count > 0);
            },
            Err(e) => {
                println!("Full RIDL Error: {}", e);
                panic!("Failed to parse full RIDL example from spec");
            }
        }
    }
    
    #[test]
    fn test_callback_types() {
        // 测试命名回调类型
        let callback_types = [
            "callback SimpleCallback();",
            "callback DataCallback(data: string | int);",
            "callback ErrorFirstCallbackType(error: string?, result: string | object?);",
            "callback ProcessCallbackType(result: string | object, success: bool, code: int);",
        ];
        
        for callback_type in callback_types {
            let result = IDLParser::parse(Rule::callback_def, callback_type);
            assert!(result.is_ok(), "Failed to parse callback: {}", callback_type);
        }
    }
    
    #[test]
    fn test_union_and_nullable_types() {
        use crate::parser::{IDLParser, Rule};
        
        let union_types = [
            "int | string",
            "bool | float | double",
            "LogLevel | string | int",
        ];
        
        for union_type in union_types {
            let result = IDLParser::parse(Rule::union_type, union_type);
            assert!(result.is_ok(), "Failed to parse union type: {}", union_type);
        }
        
        let nullable_types = [
            "string?",
            "LogLevel?",
            "map<string, object>?",
        ];
        
        for nullable_type in nullable_types {
            let result = IDLParser::parse(Rule::nullable_type, nullable_type);
            assert!(result.is_ok(), "Failed to parse nullable type: {}", nullable_type);
        }
    }
    
    #[test]
    fn test_import_and_using_statements() {
        // 测试import语句
        let import_examples = [
            "import NetworkPacket from \"Packet.proto\";",
            "import NetworkPacket as Packet from \"Packet.proto\";",
        ];
        
        for import_example in import_examples {
            let result = IDLParser::parse(Rule::import_stmt, import_example);
            assert!(result.is_ok(), "Failed to parse import: {}", import_example);
        }
        
        // 测试using语句
        let using_examples = [
            "using IntList = array<int>;",
            "using StringMap = map<string, string>;",
        ];
        
        for using_example in using_examples {
            let result = IDLParser::parse(Rule::using_stmt, using_example);
            assert!(result.is_ok(), "Failed to parse using: {}", using_example);
        }
    }
    
    #[test]
    fn test_array_and_map_types() {
        // 测试数组和映射类型
        let array_map_examples = vec![
            "array<string>",
            "array<int>",
            "array<LogEntry>",
            "map<string, string>",
            "map<string, int>",
            "map<int, string>",
            "map<string, LogEntry>",
            "array<map<string, string>>",
            "map<string, array<LogEntry>>",
        ];
        
        for type_example in array_map_examples {
            let result = IDLParser::parse(Rule::r#type, type_example);
            assert!(result.is_ok(), "Failed to parse array/map type: {}", type_example);
        }
    }
}

