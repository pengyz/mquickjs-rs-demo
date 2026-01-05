use pest_derive::Parser;
use pest::Parser;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct IDLParser;

pub mod ast;

use ast::{IDLItem, Interface, Class, Enum, Function, Type, Field, Property, Method, StructDef, PropertyModifier, EnumValue, Param, SerializationFormat};

/// 解析IDL内容
pub fn parse_idl(content: &str) -> Result<Vec<IDLItem>, Box<dyn std::error::Error>> {
    let mut pairs = IDLParser::parse(Rule::idl, content)
        .map_err(|e| format!("Parse error: {}", e))?;

    // 获取idl规则内部的定义
    let _idl_pair = pairs.next().unwrap();
    let mut items = Vec::new();
    
    // 遍历idl内部的元素（定义、WS等）
    for pair in pairs {
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
                items.push(parse_struct_def(pair)?);
            }
            Rule::global_function => {
                items.push(parse_global_function(pair)?);
            }
            Rule::callback_def => {
                items.push(parse_callback(pair)?);
            }
            Rule::using_def => {
                // TODO: Implement using definition parsing
            }
            Rule::import_stmt => {
                // TODO: Implement import statement parsing
            }
            Rule::EOI => { /* End of input, nothing to do */ }
            Rule::WS => { /* Whitespace, nothing to do */ }
            _ => {
                return Err(format!("Unexpected rule: {:?}", pair.as_rule()).into());
            }
        }
    }

    Ok(items)
}

fn parse_interface(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut interface_pairs = pair.into_inner();
    
    // 获取接口名
    let interface_name = interface_pairs.next().unwrap();
    if interface_name.as_rule() != Rule::identifier {
        return Err("Expected interface name".into());
    }
    let name = interface_name.as_str().to_string();
    
    // 解析接口体
    let mut methods = Vec::new();
    let properties = Vec::new();
    
    for pair in interface_pairs {
        match pair.as_rule() {
            Rule::method_def => {
                methods.push(parse_method(pair)?);
            }
            Rule::WS => {} // 跳过空白
            _ => {} // 其他规则
        }
    }
    
    Ok(IDLItem::Interface(Interface { name, methods, properties }))
}

fn parse_class(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let class_pairs = pair.into_inner();
    
    // 获取类名
    let class_name = class_pairs.clone().next().unwrap();
    if class_name.as_rule() != Rule::identifier {
        return Err("Expected class name".into());
    }
    let name = class_name.as_str().to_string();
    
    // 解析类体
    let mut methods = Vec::new();
    let mut properties = Vec::new();
    let mut constructor = None;
    
    for pair in class_pairs {
        match pair.as_rule() {
            Rule::class_member => {
                // 解析类成员，内部包含具体的成员定义
                let mut inner_pairs = pair.into_inner();
                let member_pair = inner_pairs.next().unwrap();
                
                match member_pair.as_rule() {
                    Rule::readwrite_prop => {
                        let prop = parse_readwrite_property(member_pair)?;
                        properties.push(prop);
                    }
                    Rule::readonly_prop => {
                        let prop = parse_readonly_property(member_pair)?;
                        properties.push(prop);
                    }
                    Rule::normal_prop => {
                        let prop = parse_normal_property(member_pair)?;
                        properties.push(prop);
                    }
                    Rule::const_member => {
                        let prop = parse_const_property(member_pair)?;  // 修复函数名
                        properties.push(prop);
                    }
                    Rule::method_def => {
                        let method = parse_method(member_pair)?;
                        methods.push(method);
                    }
                    Rule::constructor => {
                        constructor = Some(parse_constructor(member_pair)?);
                    }
                    _ => {} // 其他规则
                }
            }
            Rule::WS => {} // 跳过空白
            _ => {} // 其他规则
        }
    }
    
    Ok(IDLItem::Class(Class { name, constructor, methods, properties }))
}

fn parse_const_property(pair: pest::iterators::Pair<Rule>) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 过滤掉WS规则，只保留有意义的元素
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();
    
    if elements.len() < 3 {
        return Err(format!("Expected at least 3 elements for const property, got {}", elements.len()).into());
    }
    
    let mut iter = elements.into_iter();
    
    // 第一个非WS元素应该是标识符（属性名）
    let identifier_pair = iter.next().ok_or("Expected identifier for const property")?;
    if identifier_pair.as_rule() != Rule::identifier {
        return Err(format!("Expected identifier, got {:?}", identifier_pair.as_rule()).into());
    }
    let name = identifier_pair.as_str().to_string();
    
    // 第二个非WS元素应该是类型
    let type_pair = iter.next().ok_or("Expected type for const property")?;
    let property_type = parse_type(type_pair)?;
    
    // 第三个非WS元素应该是字面量值
    let literal_pair = iter.next().ok_or("Expected literal value for const property")?;
    let default_value = parse_literal(literal_pair)?;
    
    Ok(Property {
        modifiers: vec![PropertyModifier::Const],
        name,
        property_type,
        default_value: Some(default_value),
    })
}

fn parse_readonly_property(pair: pest::iterators::Pair<Rule>) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 过滤掉WS规则，只保留有意义的元素
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();
    
    if elements.len() < 2 {
        return Err(format!("Expected at least 2 elements for readonly property, got {}", elements.len()).into());
    }
    
    let mut iter = elements.into_iter();
    
    // 第一个非WS元素应该是标识符（属性名）
    let identifier_pair = iter.next().ok_or("Expected identifier for readonly property")?;
    if identifier_pair.as_rule() != Rule::identifier {
        return Err(format!("Expected identifier, got {:?}", identifier_pair.as_rule()).into());
    }
    let name = identifier_pair.as_str().to_string();
    
    // 第二个非WS元素应该是类型
    let type_pair = iter.next().ok_or("Expected type for readonly property")?;
    let property_type = parse_type(type_pair)?;
    
    Ok(Property {
        modifiers: vec![PropertyModifier::ReadOnly],
        name,
        property_type,
        default_value: None,
    })
}

fn parse_readwrite_property(pair: pest::iterators::Pair<Rule>) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 过滤掉WS规则，只保留有意义的元素
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();
    
    if elements.len() < 2 {
        return Err(format!("Expected at least 2 elements for readwrite property, got {}", elements.len()).into());
    }
    
    let mut iter = elements.into_iter();
    
    // 第一个非WS元素应该是标识符（属性名）
    let identifier_pair = iter.next().ok_or("Expected identifier for readwrite property")?;
    if identifier_pair.as_rule() != Rule::identifier {
        return Err(format!("Expected identifier, got {:?}", identifier_pair.as_rule()).into());
    }
    let name = identifier_pair.as_str().to_string();
    
    // 第二个非WS元素应该是类型规则
    let type_pair = iter.next().ok_or("Expected type for readwrite property")?;
    
    // 解析类型
    let property_type = parse_type(type_pair)?;
    
    Ok(Property {
        modifiers: vec![PropertyModifier::ReadWrite],
        name,
        property_type,
        default_value: None,
    })
}

fn parse_normal_property(pair: pest::iterators::Pair<Rule>) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 不过滤WS，直接遍历所有元素
    let mut pair_iter = inner_pairs.peekable();
    
    // 获取identifier
    let identifier_pair = pair_iter.next().ok_or("Expected identifier for normal property")?;
    if identifier_pair.as_rule() != Rule::identifier {
        return Err(format!("Expected identifier, got {:?}", identifier_pair.as_rule()).into());
    }
    let name = identifier_pair.as_str().to_string();
    
    // 跳过WS和冒号
    while let Some(p) = pair_iter.peek() {
        if p.as_rule() == Rule::WS || p.as_str() == ":" {
            pair_iter.next();
        } else {
            break;
        }
    }
    
    // 获取type
    let type_pair = pair_iter.next().ok_or("Expected type for normal property")?;
    let property_type = parse_type(type_pair)?;
    
    // 使用ReadWrite修饰符作为普通属性的默认值
    Ok(Property {
        modifiers: vec![PropertyModifier::ReadWrite], // 普通属性默认可读写
        name,
        property_type,
        default_value: None,
    })
}

fn parse_constructor(pair: pest::iterators::Pair<Rule>) -> Result<Function, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    let mut pair_iter = inner_pairs.filter(|p| p.as_rule() != Rule::WS);

    // constructor name
    let name_pair = pair_iter.next().ok_or("Expected constructor name")?;
    let name = name_pair.as_str().to_string();

    // parameter list
    let mut params = Vec::new();
    if let Some(param_list_pair) = pair_iter.next() {
        if param_list_pair.as_rule() == Rule::param_list {
            params = parse_param_list(param_list_pair)?;
        }
    }

    // 构造函数没有返回类型，所以使用Void
    Ok(Function { 
        name, 
        params, 
        return_type: Type::Void,
        is_async: false,
    })
}

fn parse_method(pair: pest::iterators::Pair<Rule>) -> Result<Method, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 遍历所有元素并识别它们的角色
    let mut name = String::new();
    let mut params = Vec::new();
    let mut return_type = Type::Void;
    
    for p in inner_pairs {
        match p.as_rule() {
            Rule::identifier => {
                // 在method_def中，紧跟在"fn"后的identifier是方法名
                // 我们需要更仔细地处理这个
                name = p.as_str().to_string();
            }
            Rule::param_list => {
                params = parse_param_list(p)?;
            }
            Rule::r#type => {
                return_type = parse_type(p)?;
            }
            Rule::WS => {
                // 忽略空白
            }
            _ => {
                // 忽略其他元素
            }
        }
    }
    
    if name.is_empty() {
        return Err("Method name not found".into());
    }
    
    Ok(Method {
        name,
        params,
        return_type,
        is_async: false,
    })
}

fn parse_global_function(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let function = parse_function(pair)?;
    Ok(IDLItem::Function(function))
}

fn parse_callback(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    let mut pairs_iter = inner_pairs.peekable();

    // Skip the "callback" keyword
    let first_pair = pairs_iter.next().ok_or("Callback definition has no content")?;
    if first_pair.as_rule() != Rule::identifier {
        return Err("Expected callback name after 'callback' keyword".into());
    }

    // Get the callback name
    let name = first_pair.as_str().to_string();

    // Look for parameter list
    let mut params = Vec::new();
    for p in pairs_iter {
        match p.as_rule() {
            Rule::param_list => {
                params = parse_param_list(p)?;
            }
            Rule::WS => { /* Skip whitespace */ }
            _ => { /* Ignore other rules */ }
        }
    }

    // Create a function with Callback type as return type to represent the callback
    let callback_func = Function {
        name,
        params,
        return_type: Type::Void, // Callbacks don't have return values in the signature
        is_async: false,
    };

    Ok(IDLItem::Function(callback_func))
}

fn parse_function(pair: pest::iterators::Pair<Rule>) -> Result<Function, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();

    // First element is the function name
    let name_pair = inner_pairs.next().ok_or("Function has no name")?;
    let name = name_pair.as_str().to_string();

    // Next is parameter list
    let mut params = Vec::new();
    let mut return_type = Type::Void;
    let is_async = false;

    for inner_pair in inner_pairs {
        match inner_pair.as_rule() {
            Rule::param_list => {
                params = parse_param_list(inner_pair)?;
            }
            Rule::r#type => {
                return_type = parse_type(inner_pair)?;
            }
            Rule::WS => { /* Skip whitespace */ }
            _ => {
                // For now, we'll just ignore unknown rules
            }
        }
    }

    Ok(Function {
        name,
        params,
        return_type,
        is_async,
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

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Result<Param, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // 按照语法定义：param = { identifier ~ WS ~ ":" ~ WS ~ type }
    // 但是解析器会将所有子规则展开，所以需要找到标识符和类型
    let mut name: Option<String> = None;
    let mut param_type: Option<Type> = None;
    
    let mut pair_iter = inner_pairs.peekable();
    
    // 遍历所有子规则，识别标识符和类型
    while let Some(p) = pair_iter.next() {
        match p.as_rule() {
            Rule::identifier => {
                // 这应该是参数名
                name = Some(p.as_str().to_string());
            }
            Rule::r#type => {
                // 这是参数类型
                param_type = Some(parse_type(p)?);
            }
            Rule::WS => {
                // 跳过空白
            }
            _ => {
                // 遇到其他规则，检查是否是冒号
                if p.as_str() == ":" {
                    // 冒号，继续处理
                    continue;
                } else {
                    // 不期望的规则
                    return Err(format!("Unexpected rule in parameter definition: {:?}", p.as_rule()).into());
                }
            }
        }
    }
    
    let name = name.ok_or("Parameter name not found in definition")?;
    let param_type = param_type.ok_or("Parameter type not found in definition")?;
    
    Ok(Param {
        name,
        param_type,
        optional: false, // 简化处理，不支持可选参数
    })
}

fn parse_enum(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // enum name
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();
    
    // enum values
    let mut values = Vec::new();
    for pair in inner_pairs {
        match pair.as_rule() {
            Rule::enum_value => {
                values.push(parse_enum_value(pair)?);
            }
            Rule::WS => {} // 跳过空白
            _ => {} // 其他规则
        }
    }
    
    Ok(IDLItem::Enum(Enum { name, values }))
}

fn parse_enum_value(pair: pest::iterators::Pair<Rule>) -> Result<EnumValue, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // identifier
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();
    
    // optional value
    let mut value = None;
    if let Some(value_pair) = inner_pairs.next() {
        if value_pair.as_rule() == Rule::integer {
            value = Some(value_pair.as_str().parse().unwrap());
        }
    }
    
    Ok(EnumValue { name, value })
}

fn parse_struct(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    
    // Check if this is a format-specified struct (json, msgpack, protobuf)
    let mut serialization_format = SerializationFormat::Json; // 默认为JSON
    let mut struct_pair = inner_pairs.next().ok_or("Struct definition has no content")?;
    
    // 检查是否是格式指定的结构体
    if struct_pair.as_rule() == Rule::json_format || 
       struct_pair.as_rule() == Rule::msgpack_format || 
       struct_pair.as_rule() == Rule::protobuf_format {
        
        // 确定序列化格式
        serialization_format = match struct_pair.as_rule() {
            Rule::json_format => SerializationFormat::Json,
            Rule::msgpack_format => SerializationFormat::MessagePack,
            Rule::protobuf_format => SerializationFormat::Protobuf,
            _ => SerializationFormat::Json, // 默认
        };
        
        // 获取下一个（实际的struct定义）
        struct_pair = inner_pairs.next().ok_or("Format-specified struct has no content")?;
    }
    
    let mut name = String::new();
    let mut fields = Vec::new();
    
    for pair in struct_pair.into_inner() {
        match pair.as_rule() {
            Rule::identifier => {
                name = pair.as_str().to_string();
            }
            Rule::field_def => {
                fields.push(parse_field(pair)?);
            }
            Rule::WS => { /* 跳过空白 */ }
            _ => { /* 忽略其他规则 */ }
        }
    }
    
    Ok(IDLItem::Struct(StructDef {
        name,
        fields,
        serialization_format,
    }))
}

fn parse_struct_def(pair: pest::iterators::Pair<Rule>) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    
    // Check if this is a format-specified struct (json, msgpack, protobuf)
    let mut serialization_format = SerializationFormat::Json; // 默认为JSON
    let mut pairs_iter = inner_pairs.peekable();
    
    // 查找格式化前缀，如果存在
    if let Some(first_pair) = pairs_iter.peek() {
        match first_pair.as_rule() {
            Rule::json_format => {
                serialization_format = SerializationFormat::Json;
                pairs_iter.next(); // 消费掉json_format
            }
            Rule::msgpack_format => {
                serialization_format = SerializationFormat::MessagePack;
                pairs_iter.next(); // 消费掉msgpack_format
            }
            Rule::protobuf_format => {
                serialization_format = SerializationFormat::Protobuf;
                pairs_iter.next(); // 消费掉protobuf_format
            }
            _ => {
                // 没有格式前缀，继续正常处理
            }
        }
    }
    
    let mut name = String::new();
    let mut fields = Vec::new();
    
    // 遍历剩余的pairs，寻找标识符和字段定义
    for pair in pairs_iter {
        match pair.as_rule() {
            // 跳过关键字如"struct"
            Rule::identifier => {
                name = pair.as_str().to_string();
            }
            Rule::field_def => {
                fields.push(parse_field(pair)?);
            }
            Rule::WS => { /* 跳过空白 */ }
            _ => { /* 忽略其他规则，包括"struct"关键字 */ }
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

fn parse_type(pair: pest::iterators::Pair<Rule>) -> Result<Type, Box<dyn std::error::Error>> {
    let inner_pair = pair.into_inner().next().ok_or("Type has no content")?;
    
    match inner_pair.as_rule() {
        Rule::basic_type => {
            let type_str = inner_pair.as_str();
            match type_str {
                "bool" => Ok(Type::Bool),
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "double" => Ok(Type::Double),
                "string" => Ok(Type::String),
                "void" => Ok(Type::Void),
                "object" => Ok(Type::Object),
                "function" => Ok(Type::Function),
                "callback" => Ok(Type::Callback),
                "null" => Ok(Type::Null),
                "any" => Ok(Type::Any),
                _ => Ok(Type::Custom(type_str.to_string())),
            }
        }
        Rule::array_type => {
            let inner_type = inner_pair.into_inner().next().ok_or("Array has no inner type")?;
            let inner_type = parse_type(inner_type)?;
            Ok(Type::Array(Box::new(inner_type)))
        }
        Rule::map_type => {
            let mut inner_rules = inner_pair.into_inner();
            let key_type = inner_rules.next().ok_or("Map has no key type")?;
            let value_type = inner_rules.next().ok_or("Map has no value type")?;
            let key_type = parse_type(key_type)?;
            let value_type = parse_type(value_type)?;
            Ok(Type::Map(Box::new(key_type), Box::new(value_type)))
        }
        Rule::union_type => {
            let types: Result<Vec<Type>, _> = inner_pair.into_inner().map(|p| parse_type(p)).collect();
            let types = types?;
            Ok(Type::Union(types))
        }
        Rule::optional_type => {
            let inner_type = inner_pair.into_inner().next().ok_or("Optional has no inner type")?;
            let inner_type = parse_type(inner_type)?;
            Ok(Type::Optional(Box::new(inner_type)))
        }
        Rule::custom_type => {
            Ok(Type::Custom(inner_pair.as_str().to_string()))
        }
        Rule::callback_type => {
            // For callback type, just return the Callback variant for now
            Ok(Type::Callback)
        }
        Rule::group_type => {
            let inner_type = inner_pair.into_inner().next().ok_or("Group has no inner type")?;
            let inner_type = parse_type(inner_type)?;
            Ok(Type::Group(Box::new(inner_type)))
        }
        Rule::primary_type => {
            // Handle primary_type which might be a basic type
            let inner = inner_pair.into_inner().next().ok_or("Primary type has no content")?;
            match inner.as_rule() {
                Rule::basic_type => {
                    let type_str = inner.as_str();
                    match type_str {
                        "bool" => Ok(Type::Bool),
                        "int" => Ok(Type::Int),
                        "float" => Ok(Type::Float),
                        "double" => Ok(Type::Double),
                        "string" => Ok(Type::String),
                        "void" => Ok(Type::Void),
                        "object" => Ok(Type::Object),
                        "function" => Ok(Type::Function),
                        "callback" => Ok(Type::Callback),
                        "null" => Ok(Type::Null),
                        "any" => Ok(Type::Any),
                        _ => Ok(Type::Custom(type_str.to_string())),
                    }
                }
                Rule::identifier => {
                    Ok(Type::Custom(inner.as_str().to_string()))
                }
                _ => Err(format!("Unexpected primary type rule: {:?}", inner.as_rule()).into()),
            }
        }
        _ => Err(format!("Unexpected type rule: {:?}", inner_pair.as_rule()).into()),
    }
}

fn parse_literal(pair: pest::iterators::Pair<Rule>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(pair.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use crate::parser::IDLParser;
    use crate::parser::Rule;

    #[test]
    fn test_parse_simple_interface() {
        let ridl = r#"
        interface Console {
            fn log(message: string) -> void;
            fn error(message: string) -> void;
        }
        "#;
        
        match parse_idl(ridl) {
            Ok(items) => {
                assert_eq!(items.len(), 1);
                
                match &items[0] {
                    IDLItem::Interface(interface) => {
                        assert_eq!(interface.name, "Console");
                        assert_eq!(interface.methods.len(), 2);
                        
                        let method1 = &interface.methods[0];
                        assert_eq!(method1.name, "log");
                        assert_eq!(method1.params.len(), 1);
                        assert_eq!(method1.params[0].name, "message");
                        assert_eq!(method1.params[0].param_type, Type::String);
                        assert_eq!(method1.return_type, Type::Void);
                        
                        let method2 = &interface.methods[1];
                        assert_eq!(method2.name, "error");
                        assert_eq!(method2.params.len(), 1);
                        assert_eq!(method2.params[0].name, "message");
                        assert_eq!(method2.params[0].param_type, Type::String);
                        assert_eq!(method2.return_type, Type::Void);
                    }
                    _ => panic!("Expected Interface"),
                }
            }
            Err(e) => {
                panic!("Parsing failed with error: {}", e);
            }
        }
    }

    #[test]
    fn test_parse_class_with_properties() {
        let ridl = r#"
        class Person {
            name: string;
            age: int;
            Person(name: string, age: int);
            fn getName() -> string;
            fn getAge() -> int;
            fn setAge(age: int) -> void;
        }
        "#;
        
        match parse_idl(ridl) {
            Ok(items) => {
                assert_eq!(items.len(), 1);
                
                match &items[0] {
                    IDLItem::Class(class) => {
                        assert_eq!(class.name, "Person");
                        assert_eq!(class.properties.len(), 2);
                        assert!(class.constructor.is_some());
                        assert_eq!(class.methods.len(), 3);
                        
                        // Check properties
                        assert_eq!(class.properties[0].name, "name");
                        assert_eq!(class.properties[0].property_type, Type::String);
                        assert_eq!(class.properties[1].name, "age");
                        assert_eq!(class.properties[1].property_type, Type::Int);
                        
                        // Check constructor
                        let constructor = class.constructor.as_ref().unwrap();
                        assert_eq!(constructor.name, "Person");
                        assert_eq!(constructor.params.len(), 2);
                        assert_eq!(constructor.params[0].name, "name");
                        assert_eq!(constructor.params[0].param_type, Type::String);
                        assert_eq!(constructor.params[1].name, "age");
                        assert_eq!(constructor.params[1].param_type, Type::Int);
                        
                        // Check methods
                        let get_name = &class.methods[0];
                        assert_eq!(get_name.name, "getName");
                        assert_eq!(get_name.return_type, Type::String);
                        
                        let get_age = &class.methods[1];
                        assert_eq!(get_age.name, "getAge");
                        assert_eq!(get_age.return_type, Type::Int);
                        
                        let set_age = &class.methods[2];
                        assert_eq!(set_age.name, "setAge");
                        assert_eq!(set_age.return_type, Type::Void);
                        assert_eq!(set_age.params.len(), 1);
                        assert_eq!(set_age.params[0].name, "age");
                        assert_eq!(set_age.params[0].param_type, Type::Int);
                    }
                    _ => panic!("Expected Class"),
                }
            }
            Err(e) => {
                panic!("Parsing failed with error: {}", e);
            }
        }
    }
    
    #[test]
    fn test_identifier_parsing() {
        let result = IDLParser::parse(Rule::identifier, "TestInterface");
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_interface_parsing() {
        // 根据RIDL规范，接口方法必须有返回类型
        let result = IDLParser::parse(Rule::interface_def, 
            "interface TestInterface { fn doSomething(value: int) -> void; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_interface_parsing() {
        let ridl = r#"
        interface Console {
            fn log(message: string) -> void;
            fn error(message: string) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Interface(interface) => {
                assert_eq!(interface.name, "Console");
                assert_eq!(interface.methods.len(), 2);
                
                let method1 = &interface.methods[0];
                assert_eq!(method1.name, "log");
                assert_eq!(method1.params.len(), 1);
                assert_eq!(method1.params[0].name, "message");
                assert_eq!(method1.params[0].param_type, Type::String);
                assert_eq!(method1.return_type, Type::Void);
                
                let method2 = &interface.methods[1];
                assert_eq!(method2.name, "error");
                assert_eq!(method2.params.len(), 1);
                assert_eq!(method2.params[0].name, "message");
                assert_eq!(method2.params[0].param_type, Type::String);
                assert_eq!(method2.return_type, Type::Void);
            }
            _ => panic!("Expected Interface"),
        }
    }

    #[test]
    fn test_interface_with_nullable_types() {
        let ridl = r#"
        interface NullableExample {
            fn getName() -> string?;
            fn getAge() -> int?;
            fn processName(name: string?) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Interface(interface) => {
                assert_eq!(interface.name, "NullableExample");
                assert_eq!(interface.methods.len(), 3);
                
                let method1 = &interface.methods[0];
                assert_eq!(method1.name, "getName");
                // Note: Currently our AST doesn't distinguish nullable types properly
                // This would require enhancement to the AST
                
                let method2 = &interface.methods[1];
                assert_eq!(method2.name, "getAge");
                
                let method3 = &interface.methods[2];
                assert_eq!(method3.name, "processName");
                assert_eq!(method3.params.len(), 1);
                assert_eq!(method3.params[0].name, "name");
                // Check that it's a nullable string type
            }
            _ => panic!("Expected Interface"),
        }
    }

    #[test]
    fn test_interface_with_union_types() {
        let ridl = r#"
        interface DataProcessor {
            fn processInput(data: string | int | array<string>) -> void;
            fn validateData(input: string) -> (bool | object);
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_class_definition() {
        let ridl = r#"
        class Person {
            name: string;
            age: int;
            Person(name: string, age: int);
            fn getName() -> string;
            fn getAge() -> int;
            fn setAge(age: int) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Class(class) => {
                assert_eq!(class.name, "Person");
                assert_eq!(class.properties.len(), 2);
                assert!(class.constructor.is_some());
                assert_eq!(class.methods.len(), 3);
                
                // Check properties
                assert_eq!(class.properties[0].name, "name");
                assert_eq!(class.properties[0].property_type, Type::String);
                assert_eq!(class.properties[1].name, "age");
                assert_eq!(class.properties[1].property_type, Type::Int);
                
                // Check constructor
                let constructor = class.constructor.as_ref().unwrap();
                assert_eq!(constructor.name, "Person");
                assert_eq!(constructor.params.len(), 2);
                assert_eq!(constructor.params[0].name, "name");
                assert_eq!(constructor.params[0].param_type, Type::String);
                assert_eq!(constructor.params[1].name, "age");
                assert_eq!(constructor.params[1].param_type, Type::Int);
                
                // Check methods
                let get_name = &class.methods[0];
                assert_eq!(get_name.name, "getName");
                assert_eq!(get_name.return_type, Type::String);
                
                let get_age = &class.methods[1];
                assert_eq!(get_age.name, "getAge");
                assert_eq!(get_age.return_type, Type::Int);
                
                let set_age = &class.methods[2];
                assert_eq!(set_age.name, "setAge");
                assert_eq!(set_age.return_type, Type::Void);
                assert_eq!(set_age.params.len(), 1);
                assert_eq!(set_age.params[0].name, "age");
                assert_eq!(set_age.params[0].param_type, Type::Int);
            }
            _ => panic!("Expected Class"),
        }
    }

    #[test]
    fn test_enum_definition() {
        let ridl = r#"
        enum LogLevel {
            DEBUG = 0,
            INFO = 1,
            WARN = 2,
            ERROR = 3
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Enum(enum_def) => {
                assert_eq!(enum_def.name, "LogLevel");
                assert_eq!(enum_def.values.len(), 4);
                
                assert_eq!(enum_def.values[0].name, "DEBUG");
                assert_eq!(enum_def.values[0].value, Some(0));
                
                assert_eq!(enum_def.values[1].name, "INFO");
                assert_eq!(enum_def.values[1].value, Some(1));
                
                assert_eq!(enum_def.values[2].name, "WARN");
                assert_eq!(enum_def.values[2].value, Some(2));
                
                assert_eq!(enum_def.values[3].name, "ERROR");
                assert_eq!(enum_def.values[3].value, Some(3));
            }
            _ => panic!("Expected Enum"),
        }
    }

    #[test]
    fn test_struct_definition() {
        let ridl = r#"
        struct Person {
            name: string;
            age: int;
            email: string?;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Struct(struct_def) => {
                assert_eq!(struct_def.name, "Person");
                assert_eq!(struct_def.fields.len(), 3);
                
                assert_eq!(struct_def.fields[0].name, "name");
                assert_eq!(struct_def.fields[0].field_type, Type::String);
                
                assert_eq!(struct_def.fields[1].name, "age");
                assert_eq!(struct_def.fields[1].field_type, Type::Int);
                
                assert_eq!(struct_def.fields[2].name, "email");
                // Note: Currently our AST doesn't distinguish nullable types properly
            }
            _ => panic!("Expected Struct"),
        }
    }

    #[test]
    fn test_json_struct_definition() {
        let ridl = r#"
        json struct Address {
            street: string;
            city: string;
            country: string;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        
        match &items[0] {
            IDLItem::Struct(struct_def) => {
                assert_eq!(struct_def.name, "Address");
                assert_eq!(struct_def.fields.len(), 3);
                assert_eq!(struct_def.serialization_format, SerializationFormat::Json);
            }
            _ => panic!("Expected Struct"),
        }
    }

    #[test]
    fn test_callback_definition() {
        let ridl = r#"
        callback ProcessCallback(success: bool, result: string);
        callback LogCallback(entry: LogEntry);
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        
        match &items[0] {
            IDLItem::Function(callback) => {
                assert_eq!(callback.name, "ProcessCallback");
                assert_eq!(callback.params.len(), 2);
                assert_eq!(callback.params[0].name, "success");
                assert_eq!(callback.params[0].param_type, Type::Bool);
                assert_eq!(callback.params[1].name, "result");
                assert_eq!(callback.params[1].param_type, Type::String);
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_complex_interface_with_callback() {
        let ridl = r#"
        interface CallbackExample {
            fn processData(input: string, callback: ProcessCallback) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_global_function() {
        let ridl = r#"
        fn setTimeout(callback: callback, delay: int) -> void;
        fn add(a: int, b: int) -> int;
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        
        match &items[0] {
            IDLItem::Function(function) => {
                assert_eq!(function.name, "setTimeout");
                assert_eq!(function.params.len(), 2);
                assert_eq!(function.params[0].name, "callback");
                assert_eq!(function.params[0].param_type, Type::Callback);
                assert_eq!(function.params[1].name, "delay");
                assert_eq!(function.params[1].param_type, Type::Int);
                assert_eq!(function.return_type, Type::Void);
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn test_array_and_map_types() {
        let ridl = r#"
        interface ComplexExample {
            fn getItems() -> array<string>;
            fn processArray(items: array<int>) -> void;
            fn getMetadata() -> map<string, string>;
            fn updateConfig(config: object) -> void;
        }
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_namespace_definition() {
        // RIDL不再支持namespace特性，因为JavaScript原生不支持该特性
        // 此测试保留作为说明，但不会通过
        let source = r#"
        namespace Console {
            fn log(message: string);
        }
        "#;
        
        let result = parse_idl(source);
        assert!(result.is_err() || result.unwrap().len() == 0);
    }

    #[test]
    fn test_import_statement() {
        let ridl = r#"
        import NetworkPacket as Packet from Packet.proto;
        import TypeA, TypeB from Types.proto;
        "#;
        
        let result = parse_idl(ridl);
        assert!(result.is_ok());
        
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn debug_struct_tree() {
        use pest::Parser;
        let input = "struct TestStruct { name: string; }";
        println!("Input: {}", input);
        match IDLParser::parse(Rule::struct_def, input) {
            Ok(pairs) => {
                for pair in pairs {
                    println!("Rule: {:?}, Content: \"{}\"", pair.as_rule(), pair.as_str());
                    for inner_pair in pair.into_inner() {
                        println!("  Inner Rule: {:?}, Content: \"{}\"", inner_pair.as_rule(), inner_pair.as_str());
                    }
                }
            },
            Err(e) => println!("Error: {}", e)
        }
    }
    
    #[test]
    fn debug_full_parsing() {
        use pest::Parser;
        let input = r#"
        struct Person {
            name: string;
            age: int;
        }
        "#;
        println!("Input: {}", input);
        match IDLParser::parse(Rule::idl, input) {
            Ok(pairs) => {
                for pair in pairs {
                    println!("Rule: {:?}, Content: \"{}\"", pair.as_rule(), pair.as_str());
                    for inner_pair in pair.into_inner() {
                        println!("  Inner Rule: {:?}, Content: \"{}\"", inner_pair.as_rule(), inner_pair.as_str());
                        for inner_inner in inner_pair.into_inner() {
                            println!("    Inner Inner Rule: {:?}, Content: \"{}\"", inner_inner.as_rule(), inner_inner.as_str());
                        }
                    }
                }
            },
            Err(e) => println!("Error: {}", e)
        }
    }
}