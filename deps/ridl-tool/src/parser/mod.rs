use crate::parser::ast::{Import, Singleton, Using};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct IDLParser;

pub mod ast;

use ast::{
    Class, Enum, EnumValue, Field, Function, IDLItem, Interface, Method, ModuleDeclaration, Param,
    Property, PropertyModifier, SerializationFormat, StructDef, Type,
};

fn pair_pos(pair: &pest::iterators::Pair<Rule>) -> ast::SourcePos {
    let (line, column) = pair.line_col();
    ast::SourcePos { line, column }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    Default,
    Strict,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedIDL {
    pub module: Option<ModuleDeclaration>,
    pub mode: FileMode,
    pub items: Vec<IDLItem>,
}

/// 解析IDL内容
pub fn parse_idl(content: &str) -> Result<Vec<IDLItem>, Box<dyn std::error::Error>> {
    Ok(parse_idl_file(content)?.items)
}

/// 解析IDL内容并携带文件级 mode 信息
pub fn parse_idl_file(content: &str) -> Result<ParsedIDL, Box<dyn std::error::Error>> {
    let mut pairs =
        IDLParser::parse(Rule::idl, content).map_err(|e| format!("Parse error: {}", e))?;

    // 获取idl规则内部的定义
    let idl_pair = pairs.next().unwrap();
    let mut items = Vec::new();
    let mut module: Option<ModuleDeclaration> = None;
    let mut mode: FileMode = FileMode::Default;

    // 遍历idl内部的元素
    for pair in idl_pair.into_inner() {
        match pair.as_rule() {
            Rule::mode_decl => {
                mode = parse_mode_decl(pair)?;
            }
            Rule::module_decl => {
                // 解析模块声明，只在开头出现
                module = Some(parse_module_decl(pair)?);
            }
            Rule::definition => {
                // 解析定义内部的具体定义类型
                let inner_pair = pair.into_inner().next().ok_or("Definition is empty")?;
                let item = parse_definition_content(inner_pair, module.clone())?;
                items.push(item);
            }
            Rule::EOI => { /* End of input, nothing to do */ }
            Rule::WS => { /* Whitespace, nothing to do */ }
            _ => {
                return Err(format!("Unexpected rule at top level: {:?}", pair.as_rule()).into());
            }
        }
    }

    Ok(ParsedIDL { module, mode, items })
}

fn parse_mode_decl(
    pair: pest::iterators::Pair<Rule>,
) -> Result<FileMode, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();
    let mode_name_pair = inner_pairs.next().ok_or("Mode declaration has no name")?;
    let name = mode_name_pair.as_str();

    match name {
        "strict" => Ok(FileMode::Strict),
        _ => Err(format!("Unknown mode: {name}").into()),
    }
}

/// 解析RIDL内容（与parse_idl相同，用于API一致性）
pub fn parse_ridl(content: &str) -> Result<Vec<IDLItem>, Box<dyn std::error::Error>> {
    parse_idl(content)
}

/// 解析RIDL内容并携带文件级 mode 信息
pub fn parse_ridl_file(content: &str) -> Result<ParsedIDL, Box<dyn std::error::Error>> {
    parse_idl_file(content)
}

fn decode_ridl_string_literal(
    pos: &crate::parser::ast::SourcePos,
    raw: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| {
            format!(
                "Invalid string literal at {}:{}: missing surrounding quotes",
                pos.line, pos.column
            )
        })?;

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        // In pest span, a RIDL escape like `\n` is observed as `\\n`.
        // That means:
        // - an actual escape prefix is represented by a *pair* of backslashes in `inner`.
        // - a literal backslash character is also represented by a pair, and is expressed
        //   as the escape `\\\\` (i.e. `"\\\\"` inside the source string literal).
        let mut run = 1usize;
        while let Some('\\') = chars.peek().copied() {
            chars.next();
            run += 1;
        }

        // Our pest span may contain an odd run right before `"` for the sequence `\\\"`
        // (bytes [92,92,92,34]). This represents the escape `\"` (a quote) in RIDL.
        let pairs = run / 2;
        let odd = run % 2 == 1;

        let mut next = chars.peek().copied();

        if odd {
            if next == Some('"') {
                // `\\\"` => decode to `"`.
                let esc = chars.next().ok_or_else(|| {
                    format!(
                        "Invalid string literal at {}:{}: trailing escape\\",
                        pos.line, pos.column
                    )
                })?;
                debug_assert_eq!(esc, '"');
                out.push('"');
                continue;
            }

            return Err(format!(
                "Invalid string literal at {}:{}: invalid backslash sequence",
                pos.line, pos.column
            )
            .into());
        }

        next = chars.peek().copied();

        if matches!(next, Some('n' | 't' | 'r' | '"')) {
            for _ in 0..(pairs - 1) {
                out.push('\\');
            }

            let esc = chars.next().ok_or_else(|| {
                format!(
                    "Invalid string literal at {}:{}: trailing escape\\",
                    pos.line, pos.column
                )
            })?;

            match esc {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                _ => unreachable!(),
            }

            continue;
        }

        if matches!(next, Some('\\')) {
            // Decode `\\\\` as a single literal backslash.
            // In inner this can show up as 4 backslashes followed by `\\` (next='\\'),
            // so we need at least 2 pairs.
            if pairs < 2 {
                return Err(format!(
                    "Invalid string literal at {}:{}: invalid backslash sequence",
                    pos.line, pos.column
                )
                .into());
            }

            for _ in 0..(pairs - 2) {
                out.push('\\');
            }

            let esc = chars.next().ok_or_else(|| {
                format!(
                    "Invalid string literal at {}:{}: trailing escape\\",
                    pos.line, pos.column
                )
            })?;
            debug_assert_eq!(esc, '\\');
            out.push('\\');
            continue;
        }

        if pairs == 1 {
            let next = chars.peek().copied().unwrap_or('\0');
            return Err(format!(
                "Invalid string literal at {}:{}: unsupported escape \\\\{}",
                pos.line, pos.column, next
            )
            .into());
        }

        for _ in 0..pairs {
            out.push('\\');
        }
    }

    Ok(out)
}

fn parse_module_decl(
    pair: pest::iterators::Pair<Rule>,
) -> Result<ModuleDeclaration, Box<dyn std::error::Error>> {
    let pos = Some(pair_pos(&pair));
    let mut inner_pairs = pair.into_inner();

    // module_path
    let module_path_pair = inner_pairs.next().ok_or("Module declaration has no path")?;
    let module_path = parse_module_path(module_path_pair)?;

    // version (optional)
    let mut version = None;
    if let Some(version_pair) = inner_pairs.next() {
        version = Some(version_pair.as_str().to_string());
    }

    Ok(ModuleDeclaration {
        module_path,
        version,
        pos,
    })
}

fn parse_module_path(
    pair: pest::iterators::Pair<Rule>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut path_parts = Vec::new();

    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::identifier {
            path_parts.push(inner_pair.as_str().to_string());
        }
    }

    Ok(path_parts.join("."))
}

fn parse_definition_content(
    pair: pest::iterators::Pair<Rule>,
    module: Option<ModuleDeclaration>,
) -> Result<IDLItem, Box<dyn std::error::Error>> {
    match pair.as_rule() {
        Rule::interface_def => {
            let mut interface = parse_interface(pair)?;
            interface.module = module;
            Ok(IDLItem::Interface(interface))
        }
        Rule::class_def => {
            let mut class = parse_class(pair)?;
            class.module = module;
            Ok(IDLItem::Class(class))
        }
        Rule::enum_def => {
            let mut enum_def = parse_enum(pair)?;
            enum_def.module = module;
            Ok(IDLItem::Enum(enum_def))
        }
        Rule::struct_def => {
            let mut struct_def = parse_struct_def(pair)?;
            struct_def.module = module;
            Ok(IDLItem::Struct(struct_def))
        }
        Rule::global_function => {
            let mut function = parse_global_function(pair)?;
            match &mut function {
                IDLItem::Function(f) => f.module = module,
                _ => {}
            }
            Ok(function)
        }
        Rule::callback_def => parse_callback(pair),
        Rule::using_def => {
            let mut using = parse_using(pair)?;
            using.module = module;
            Ok(IDLItem::Using(using))
        }
        Rule::import_stmt => {
            let mut import = parse_import(pair)?;
            import.module = module;
            Ok(IDLItem::Import(import))
        }
        Rule::singleton_def => {
            let mut singleton = parse_singleton(pair)?;
            singleton.module = module;
            Ok(IDLItem::Singleton(singleton))
        }
        _ => Err(format!("Unexpected definition content: {:?}", pair.as_rule()).into()),
    }
}

fn parse_interface(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Interface, Box<dyn std::error::Error>> {
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
            _ => {}        // 其他规则
        }
    }

    Ok(Interface {
        name,
        methods,
        properties,
        module: None,
    })
}

fn parse_class(pair: pest::iterators::Pair<Rule>) -> Result<Class, Box<dyn std::error::Error>> {
    let pos = Some(pair_pos(&pair));
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
    let mut js_fields = Vec::new();
    let mut constructor = None;

    for pair in class_pairs {
        match pair.as_rule() {
            Rule::class_member => {
                // 解析类成员，内部包含具体的成员定义
                let mut inner_pairs = pair.into_inner();
                let member_pair = inner_pairs.next().unwrap();

                match member_pair.as_rule() {
                    Rule::proto_readwrite_prop => {
                        let mut prop = parse_readwrite_property(member_pair)?;
                        prop.modifiers.insert(0, PropertyModifier::Proto);
                        properties.push(prop);
                    }
                    Rule::proto_readonly_prop => {
                        let mut prop = parse_readonly_property(member_pair)?;
                        prop.modifiers.insert(0, PropertyModifier::Proto);
                        properties.push(prop);
                    }
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
                        return Err("`const` is not supported in mquickjs RIDL; use `var`".into());
                    }
                    Rule::var_member => {
                        let f = parse_var_field(member_pair)?;
                        js_fields.push(f);
                    }
                    Rule::proto_var_member => {
                        let mut f = parse_var_field(member_pair)?;
                        f.modifiers.insert(0, crate::parser::ast::PropertyModifier::Proto);
                        js_fields.push(f);
                    }
                    Rule::method_def => {
                        let method = parse_method(member_pair)?;
                        methods.push(method);
                    }
                    Rule::class_constructor => {
                        constructor = Some(parse_class_constructor(member_pair, &name)?);
                    }
                    Rule::class_constructor_compat => {
                        constructor = Some(parse_class_constructor(member_pair, &name)?);
                    }
                    _ => {} // 其他规则
                }
            }
            Rule::WS => {} // 跳过空白
            _ => {}        // 其他规则
        }
    }

    Ok(Class {
        name,
        pos,
        constructor,
        methods,
        properties,
        js_fields,
        module: None,
    })
}

fn parse_var_field(
    pair: pest::iterators::Pair<Rule>,
) -> Result<crate::parser::ast::JsField, Box<dyn std::error::Error>> {
    let pos = Some(pair_pos(&pair));
    let inner_pairs = pair.into_inner();
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();

    if elements.len() < 3 {
        return Err(format!(
            "Expected at least 3 elements for var field, got {}",
            elements.len()
        )
        .into());
    }

    let mut iter = elements.into_iter();
    let name_pair = iter.next().ok_or("Expected identifier for var field")?;
    let name = name_pair.as_str().to_string();

    let type_pair = iter.next().ok_or("Expected type for var field")?;
    let property_type = parse_type(type_pair)?;

    let literal_pair = iter
        .next()
        .ok_or("Expected literal value for var field")?;

    let init_literal = match property_type {
        crate::parser::ast::Type::String => decode_ridl_string_literal(&pair_pos(&literal_pair), literal_pair.as_str())?,
        _ => literal_pair.as_str().to_string(),
    };

    Ok(crate::parser::ast::JsField {
        kind: crate::parser::ast::JsFieldKind::Var,
        modifiers: Vec::new(),
        name,
        pos,
        field_type: property_type,
        init_literal,
    })
}

fn parse_readonly_property(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();

    // 过滤掉WS规则，只保留有意义的元素
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();

    if elements.len() < 2 {
        return Err(format!(
            "Expected at least 2 elements for readonly property, got {}",
            elements.len()
        )
        .into());
    }

    let mut iter = elements.into_iter();

    // 第一个非WS元素应该是标识符（属性名）
    let identifier_pair = iter
        .next()
        .ok_or("Expected identifier for readonly property")?;
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
    })
}

fn parse_readwrite_property(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();

    // 过滤掉WS规则，只保留有意义的元素
    let elements: Vec<_> = inner_pairs.filter(|p| p.as_rule() != Rule::WS).collect();

    if elements.len() < 2 {
        return Err(format!(
            "Expected at least 2 elements for readwrite property, got {}",
            elements.len()
        )
        .into());
    }

    let mut iter = elements.into_iter();

    // 第一个非WS元素应该是标识符（属性名）
    let identifier_pair = iter
        .next()
        .ok_or("Expected identifier for readwrite property")?;
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
    })
}

fn parse_normal_property(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Property, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();

    // 不过滤WS，直接遍历所有元素
    let mut pair_iter = inner_pairs.peekable();

    // 获取identifier
    let identifier_pair = pair_iter
        .next()
        .ok_or("Expected identifier for normal property")?;
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
    let type_pair = pair_iter
        .next()
        .ok_or("Expected type for normal property")?;
    let property_type = parse_type(type_pair)?;

    // 使用ReadWrite修饰符作为普通属性的默认值
    Ok(Property {
        modifiers: vec![PropertyModifier::ReadWrite], // 普通属性默认可读写
        name,
        property_type,
    })
}

fn parse_class_constructor(
    pair: pest::iterators::Pair<Rule>,
    class_name: &str,
) -> Result<Function, Box<dyn std::error::Error>> {
    let rule = pair.as_rule();
    let inner_pairs = pair.into_inner();
    let mut pair_iter = inner_pairs.filter(|p| p.as_rule() != Rule::WS);

    // Preferred: constructor(<params>)
    // Compat: <ClassName>(<params>)
    let name: String;
    if rule == Rule::class_constructor_compat {
        let name_pair = pair_iter.next().ok_or("Expected constructor name")?;
        name = name_pair.as_str().to_string();
        if name != class_name {
            return Err(format!(
                "constructor compat form must match class name: expected {}, got {}",
                class_name, name
            )
            .into());
        }
    } else {
        // Use a stable internal name; glue generation does not need it.
        name = "constructor".to_string();
    }

    // parameter list
    let mut params = Vec::new();
    if let Some(param_list_pair) = pair_iter.next() {
        if param_list_pair.as_rule() == Rule::param_list {
            params = parse_param_list(param_list_pair)?;
        }
    }

    Ok(Function {
        name,
        params,
        return_type: Type::Void,
        is_async: false,
        module: None,
    })
}

fn parse_method(pair: pest::iterators::Pair<Rule>) -> Result<Method, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();

    let mut name = String::new();
    let mut params = Vec::new();
    let mut return_type = Type::Void;

    for p in inner_pairs {
        match p.as_rule() {
            Rule::identifier => {
                // identifier可能是方法名或参数名，我们需要更仔细地处理
                if name.is_empty() {
                    // 如果name还没有被设置，这很可能是方法名（紧跟在fn之后）
                    name = p.as_str().to_string();
                }
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

fn parse_global_function(
    pair: pest::iterators::Pair<Rule>,
) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let function = parse_function(pair)?;
    Ok(IDLItem::Function(function))
}

fn parse_function(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Function, Box<dyn std::error::Error>> {
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
        module: None,
    })
}

fn parse_param_list(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<Param>, Box<dyn std::error::Error>> {
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

    // 语义约束：variadic 参数必须是最后一个
    if let Some(last_variadic_idx) = params.iter().position(|p| p.variadic) {
        if last_variadic_idx != params.len() - 1 {
            return Err("Variadic parameter must be the last parameter".into());
        }
        if params
            .iter()
            .skip(last_variadic_idx + 1)
            .any(|p| p.variadic)
        {
            return Err("Only one variadic parameter is allowed".into());
        }
    }

    Ok(params)
}

fn parse_param(pair: pest::iterators::Pair<Rule>) -> Result<Param, Box<dyn std::error::Error>> {
    match pair.as_rule() {
        Rule::param => {
            // param = { variadic_param | normal_param }
            let inner = pair
                .into_inner()
                .next()
                .ok_or("Parameter definition is empty")?;
            parse_param(inner)
        }
        Rule::normal_param | Rule::variadic_param => {
            let variadic = matches!(pair.as_rule(), Rule::variadic_param);
            let mut inner_pairs = pair.into_inner();

            let name_pair = inner_pairs
                .next()
                .ok_or("Parameter name not found in definition")?;
            if name_pair.as_rule() != Rule::identifier {
                return Err("Parameter name not found in definition".into());
            }
            let name = name_pair.as_str().to_string();

            let type_pair = inner_pairs
                .next()
                .ok_or("Parameter type not found in definition")?;
            if type_pair.as_rule() != Rule::r#type {
                return Err("Parameter type not found in definition".into());
            }
            let param_type = parse_type(type_pair)?;

            Ok(Param {
                name,
                param_type,
                optional: false,
                variadic,
            })
        }
        _ => Err(format!("Unexpected rule for param: {:?}", pair.as_rule()).into()),
    }
}

fn parse_enum(pair: pest::iterators::Pair<Rule>) -> Result<Enum, Box<dyn std::error::Error>> {
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
            _ => {}        // 其他规则
        }
    }

    Ok(Enum {
        name,
        values,
        module: None,
    })
}

fn parse_enum_value(
    pair: pest::iterators::Pair<Rule>,
) -> Result<EnumValue, Box<dyn std::error::Error>> {
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

fn parse_struct_def(
    pair: pest::iterators::Pair<Rule>,
) -> Result<StructDef, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();

    // Check if this is a format-specified struct (json, msgpack, protobuf)
    let mut serialization_format = SerializationFormat::Json; // 默认为JSON
    let mut pairs_iter = inner_pairs.peekable();

    // 查找格式化前缀，如果存在
    if let Some(first_pair) = pairs_iter.peek() {
        if first_pair.as_str().contains("json") {
            serialization_format = SerializationFormat::Json;
            pairs_iter.next(); // 消费掉格式前缀
        } else if first_pair.as_str().contains("msgpack") {
            serialization_format = SerializationFormat::MessagePack;
            pairs_iter.next(); // 消费掉格式前缀
        } else if first_pair.as_str().contains("protobuf") {
            serialization_format = SerializationFormat::Protobuf;
            pairs_iter.next(); // 消费掉格式前缀
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

    Ok(StructDef {
        name,
        fields,
        serialization_format,
        module: None,
    })
}

fn parse_field(pair: pest::iterators::Pair<Rule>) -> Result<Field, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();

    // identifier
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();

    // 跳过冒号和可能的空白
    let mut type_pair = None;
    for p in inner_pairs {
        if p.as_rule() == Rule::r#type
            || p.as_rule() == Rule::primary_type
            || p.as_rule() == Rule::basic_type
            || p.as_rule() == Rule::custom_type
            || p.as_rule() == Rule::array_type
            || p.as_rule() == Rule::map_type
            || p.as_rule() == Rule::callback_type
            || p.as_rule() == Rule::group_type
            || p.as_rule() == Rule::nullable_type
            || p.as_rule() == Rule::union_type
        {
            type_pair = Some(p);
            break;
        }
    }

    let field_type = match type_pair {
        Some(tp) => parse_type(tp)?,
        None => return Err("Field has no type".into()),
    };

    Ok(Field {
        name,
        field_type,
        optional: false, // 简化处理，不支持可选字段
    })
}

fn parse_type(pair: pest::iterators::Pair<Rule>) -> Result<Type, Box<dyn std::error::Error>> {
    // 检查是否是nullable类型
    if pair.as_rule() == Rule::nullable_type {
        return parse_nullable_type(pair);
    }

    // 检查是否是union类型
    if pair.as_rule() == Rule::union_type {
        return parse_union_type(pair);
    }

    // 检查是否有子规则，优先处理子规则
    for inner_pair in pair.clone().into_inner() {
        match inner_pair.as_rule() {
            Rule::basic_type => {
                let type_str = inner_pair.as_str();
                return match type_str {
                    "bool" => Ok(Type::Bool),
                    "int" => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "double" => Ok(Type::Double),
                    "string" => Ok(Type::String),
                    "void" => Ok(Type::Void),
                    "object" => Ok(Type::Object),
                    "null" => Ok(Type::Null),
                    "any" => Ok(Type::Any),
                    _ => Ok(Type::Custom(type_str.to_string())),
                };
            }
            Rule::array_type => {
                for p in inner_pair.into_inner() {
                    if p.as_rule() != Rule::WS && p.as_str() != "array" {
                        let inner_type = parse_type(p)?;
                        return Ok(Type::Array(Box::new(inner_type)));
                    }
                }
                return Err("Array has no inner type".into());
            }
            Rule::map_type => {
                let mut types = Vec::new();

                for p in inner_pair.into_inner() {
                    if p.as_rule() != Rule::WS
                        && p.as_str() != "<"
                        && p.as_str() != ">"
                        && p.as_str() != ","
                    {
                        // 这应该是key或value类型
                        let inner_type = parse_type(p)?;
                        types.push(inner_type);
                    }
                }

                if types.len() >= 2 {
                    return Ok(Type::Map(
                        Box::new(types[0].clone()),
                        Box::new(types[1].clone()),
                    ));
                } else {
                    return Err("Map has insufficient type parameters".into());
                }
            }
            Rule::union_type => {
                return parse_union_type(inner_pair);
            }
            Rule::nullable_type => {
                // 修复：添加对内部nullable_type的处理
                return parse_nullable_type(inner_pair);
            }
            Rule::custom_type => {
                return Ok(Type::Custom(inner_pair.as_str().to_string()));
            }
            Rule::callback_type => {
                let mut params = Vec::new();

                for p in inner_pair.into_inner() {
                    match p.as_rule() {
                        Rule::param_list => {
                            params = parse_param_list(p)?;
                        }
                        Rule::WS => { /* 跳过空白 */ }
                        _ => { /* 忽略其他规则 */ }
                    }
                }

                return Ok(Type::CallbackWithParams(params));
            }
            Rule::group_type => {
                for p in inner_pair.into_inner() {
                    if p.as_rule() != Rule::WS {
                        let inner_type = parse_type(p)?;
                        return Ok(Type::Group(Box::new(inner_type)));
                    }
                }
                return Err("Group has no inner type".into());
            }
            Rule::primary_type => {
                // 递归解析primary_type
                return parse_type(inner_pair);
            }
            _ => continue, // 对于其他内部规则，继续处理
        }
    }

    // 如果没有内部规则，则检查当前规则
    match pair.as_rule() {
        Rule::basic_type => {
            let type_str = pair.as_str();
            match type_str {
                "bool" => Ok(Type::Bool),
                "int" => Ok(Type::Int),
                "float" => Ok(Type::Float),
                "double" => Ok(Type::Double),
                "string" => Ok(Type::String),
                "void" => Ok(Type::Void),
                "object" => Ok(Type::Object),
                "null" => Ok(Type::Null),
                "any" => Ok(Type::Any),
                _ => Ok(Type::Custom(type_str.to_string())),
            }
        }
        Rule::custom_type => Ok(Type::Custom(pair.as_str().to_string())),
        _ => {
            // 如果无法识别，返回自定义类型
            Ok(Type::Custom(pair.as_str().to_string()))
        }
    }
}

fn parse_union_type(pair: pest::iterators::Pair<Rule>) -> Result<Type, Box<dyn std::error::Error>> {
    let mut types = Vec::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::WS => { /* 跳过空白 */ }
            _ => {
                let parsed_type = parse_type(inner_pair)?;
                types.push(parsed_type);
            }
        }
    }

    if types.is_empty() {
        return Err("Union type has no types".into());
    }

    Ok(Type::Union(types))
}

fn parse_nullable_type(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Type, Box<dyn std::error::Error>> {
    // nullable_type由一个基础类型和?组成
    let inner_pairs = pair.into_inner();

    // 找到基础类型
    for inner_pair in inner_pairs {
        match inner_pair.as_rule() {
            Rule::basic_type
            | Rule::array_type
            | Rule::map_type
            | Rule::custom_type
            | Rule::callback_type
            | Rule::group_type => {
                let base_type = parse_type(inner_pair)?;
                return Ok(Type::Optional(Box::new(base_type)));
            }
            Rule::WS => { /* 跳过空白 */ }
            _ => { /* 其他规则，继续寻找类型 */ }
        }
    }

    Err("Nullable type has no base type".into())
}

#[allow(dead_code)]
fn parse_literal(pair: pest::iterators::Pair<Rule>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(pair.as_str().to_string())
}

fn parse_using(pair: pest::iterators::Pair<Rule>) -> Result<Using, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();

    // identifier
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();

    // type
    let type_pair = inner_pairs.next().unwrap();
    let alias_type = parse_type(type_pair)?;

    Ok(Using {
        name,
        alias_type,
        module: None,
    })
}

fn parse_import(pair: pest::iterators::Pair<Rule>) -> Result<Import, Box<dyn std::error::Error>> {
    let mut inner_pairs = pair.into_inner();

    // import_list
    let import_list_pair = inner_pairs.next().unwrap();
    let imports = parse_import_list(import_list_pair)?;

    // string_literal
    let path_pair = inner_pairs.next().unwrap();
    let path = path_pair.as_str().trim_matches('"').to_string();

    Ok(Import {
        imports,
        path,
        module: None,
    })
}

fn parse_import_list(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<ast::ImportItem>, Box<dyn std::error::Error>> {
    let mut imports = Vec::new();
    let inner_pairs = pair.into_inner();

    for p in inner_pairs {
        match p.as_rule() {
            Rule::identifier => {
                imports.push(ast::ImportItem {
                    name: p.as_str().to_string(),
                    alias: None,
                });
            }
            Rule::import_list => {
                let mut item_pairs = p.into_inner();

                while item_pairs.peek().is_some() {
                    let name_pair = item_pairs.next().unwrap();
                    let name = name_pair.as_str().to_string();

                    let alias_pair = item_pairs.next();
                    let alias = if let Some(alias_pair) = alias_pair {
                        Some(alias_pair.as_str().to_string())
                    } else {
                        None
                    };

                    imports.push(ast::ImportItem { name, alias });
                }
            }
            Rule::import_stmt => {
                let mut item_pairs = p.into_inner();

                let alias_pair = item_pairs.next().unwrap();
                let alias = alias_pair.as_str().to_string();

                imports.push(ast::ImportItem {
                    name: "*".to_string(),
                    alias: Some(alias),
                });
            }
            _ => {}
        }
    }

    Ok(imports)
}

fn parse_singleton(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Singleton, Box<dyn std::error::Error>> {
    let pos = Some(pair_pos(&pair));
    let mut inner_pairs = pair.into_inner();

    // identifier
    let name_pair = inner_pairs.next().unwrap();
    let name = name_pair.as_str().to_string();

    // singleton body
    let mut methods = Vec::new();
    let mut properties = Vec::new();

    for p in inner_pairs {
        match p.as_rule() {
            Rule::singleton_member => {
                let mut member_pairs = p.into_inner();
                let member_pair = member_pairs.next().unwrap();

                match member_pair.as_rule() {
                    Rule::method_def => {
                        let method = parse_method(member_pair)?;
                        methods.push(method);
                    }
                    Rule::readonly_prop => {
                        let prop = parse_readonly_property(member_pair)?;
                        properties.push(prop);
                    }
                    Rule::readwrite_prop => {
                        let prop = parse_readwrite_property(member_pair)?;
                        properties.push(prop);
                    }
                    Rule::normal_prop => {
                        return Err("`normal_prop` is not supported in singleton; use `property`".into());
                    }
                    _ => {}
                }
            }
            Rule::WS => {}
            _ => {}
        }
    }

    Ok(Singleton {
        name,
        pos,
        methods,
        properties,
        module: None,
    })
}

fn parse_callback(
    pair: pest::iterators::Pair<Rule>,
) -> Result<IDLItem, Box<dyn std::error::Error>> {
    let inner_pairs = pair.into_inner();
    let pairs_iter = inner_pairs.peekable();

    // 跳过"callback"关键字，获取回调名（可选）
    let mut name = String::from("anonymous_callback"); // 默认名称
    let mut has_processed_first = false;
    let mut params = Vec::new();

    for p in pairs_iter {
        match p.as_rule() {
            Rule::identifier => {
                if !has_processed_first {
                    name = p.as_str().to_string();
                    has_processed_first = true;
                }
            }
            Rule::param_list => {
                params = parse_param_list(p)?;
            }
            Rule::WS => { /* Skip whitespace */ }
            _ => { /* Ignore other rules */ }
        }
    }

    // 创建回调函数
    let callback_func = Function {
        name,
        params,
        return_type: Type::Void, // 回调函数没有返回值
        is_async: false,
        module: None,
    };

    Ok(IDLItem::Function(callback_func))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::IDLParser;
    use crate::parser::Rule;
    use pest::Parser;

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
            var name: string = "";
            var age: int = 0;
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
                        assert_eq!(class.js_fields.len(), 2);
                        assert!(class.constructor.is_some());
                        assert_eq!(class.methods.len(), 3);

                        let f1 = &class.js_fields[0];
                        assert_eq!(f1.name, "name");
                        assert_eq!(f1.field_type, Type::String);

                        let f2 = &class.js_fields[1];
                        assert_eq!(f2.name, "age");
                        assert_eq!(f2.field_type, Type::Int);

                        let constructor = class.constructor.as_ref().unwrap();
                        assert_eq!(constructor.name, "Person");
                        assert_eq!(constructor.params.len(), 2);

                        let method1 = &class.methods[0];
                        assert_eq!(method1.name, "getName");
                        assert_eq!(method1.return_type, Type::String);
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
    fn test_parse_complex_types() {
        let ridl = r#"
        interface Test {
            fn testFn(cb: callback(success: bool, result: string)) -> void;
            fn handleNullable(data: string?) -> void;
            fn handleUnion(data: (bool | object)) -> void;
            fn handleMap(data: map<string, int>) -> void;
            fn handleArray(data: array<string>) -> void;
            fn handleOptionalParam(data: string?) -> void;
            fn setCallback(cb: callback(success: bool)) -> void;  // 修复：使用cb而不是callback作为方法名
        }
        "#;

        match parse_idl(ridl) {
            Ok(items) => {
                if let IDLItem::Interface(interface) = &items[0] {
                    // 检查testFn方法
                    let test_fn = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "testFn")
                        .unwrap();
                    if let Type::CallbackWithParams(params) = &test_fn.params[0].param_type {
                        assert_eq!(params.len(), 2);
                        assert_eq!(params[0].name, "success");
                        assert_eq!(params[0].param_type, Type::Bool);
                        assert_eq!(params[1].name, "result");
                        assert_eq!(params[1].param_type, Type::String);
                    } else {
                        panic!("Expected callback type with params");
                    }

                    // 检查handleNullable方法 - 返回类型是void，不是Optional(void)
                    let handle_nullable = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "handleNullable")
                        .unwrap();
                    if let Type::Optional(inner_type) = &handle_nullable.params[0].param_type {
                        assert_eq!(**inner_type, Type::String);
                    } else {
                        panic!(
                            "Expected optional parameter type, got {:?}",
                            handle_nullable.params[0].param_type
                        );
                    }

                    // 检查handleUnion方法 - 参数是联合类型，不是返回类型
                    let handle_union = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "handleUnion")
                        .unwrap();
                    let param_type = match &handle_union.params[0].param_type {
                        Type::Group(inner) => &**inner, // 如果是分组的，解包
                        other => other,                 // 否则直接使用
                    };
                    if let Type::Union(types) = param_type {
                        assert_eq!(types.len(), 2);
                        assert!(types.iter().any(|t| matches!(t, Type::Bool)));
                        assert!(types.iter().any(|t| matches!(t, Type::Object)));
                    } else {
                        panic!(
                            "Expected union parameter type, got {:?}",
                            handle_union.params[0].param_type
                        );
                    }

                    // 检查handleMap方法
                    let handle_map = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "handleMap")
                        .unwrap();
                    if let Type::Map(key_type, value_type) = &handle_map.params[0].param_type {
                        assert_eq!(**key_type, Type::String);
                        assert_eq!(**value_type, Type::Int);
                    } else {
                        panic!(
                            "Expected map parameter type, got {:?}",
                            handle_map.params[0].param_type
                        );
                    }

                    // 检查handleArray方法
                    let handle_array = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "handleArray")
                        .unwrap();
                    if let Type::Array(inner_type) = &handle_array.params[0].param_type {
                        assert_eq!(**inner_type, Type::String);
                    } else {
                        panic!(
                            "Expected array parameter type, got {:?}",
                            handle_array.params[0].param_type
                        );
                    }

                    // 检查handleOptionalParam方法
                    let handle_optional = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "handleOptionalParam")
                        .unwrap();
                    if let Type::Optional(inner_type) = &handle_optional.params[0].param_type {
                        assert_eq!(**inner_type, Type::String);
                    } else {
                        panic!(
                            "Expected optional parameter type, got {:?}",
                            handle_optional.params[0].param_type
                        );
                    }

                    // 检查setCallback方法
                    let set_callback = &interface
                        .methods
                        .iter()
                        .find(|m| m.name == "setCallback")
                        .unwrap();
                    if let Type::CallbackWithParams(params) = &set_callback.params[0].param_type {
                        assert_eq!(params.len(), 1);
                        assert_eq!(params[0].name, "success");
                        assert_eq!(params[0].param_type, Type::Bool);
                    } else {
                        panic!("Expected callback type for setCallback param");
                    }
                }
            }
            Err(e) => panic!("Complex types parsing failed: {}", e),
        }
    }

    #[test]
    fn test_parse_module_declaration() {
        let ridl = r#"
        module system.network@1.0
        interface Network {
            fn getStatus() -> string;
        }
        "#;

        match parse_idl(ridl) {
            Ok(items) => {
                assert_eq!(items.len(), 1);

                match &items[0] {
                    IDLItem::Interface(interface) => {
                        assert_eq!(interface.name, "Network");
                        assert!(interface.module.is_some());

                        let module = interface.module.as_ref().unwrap();
                        assert_eq!(module.module_path, "system.network");
                        assert_eq!(module.version, Some("1.0".to_string()));
                    }
                    _ => panic!("Expected Interface with module"),
                }
            }
            Err(e) => {
                panic!("Parsing module declaration failed with error: {}", e);
            }
        }
    }

    #[test]
    fn test_comprehensive_ridl_syntax() {
        let ridl = r#"
        module std.console@1.0

        // 使用定义
        using StringMap = map<string, string>;

        // 枚举定义
        enum LogLevel {
            DEBUG,
            INFO,
            WARN,
            ERROR
        }

        // 结构体定义
        struct LogEntry {
            level: LogLevel;
            message: string;
            timestamp: int;
            metadata: map<string, string>;
            tags: array<string>;
            callback_func: callback(success: bool, result: string);
        }

        // 接口定义
        interface Console {
            fn log(message: string) -> void;
            fn logWithLevel(message: string, level: LogLevel) -> void;
            fn error(message: string?) -> void;  // 可空类型
            fn processMultiple(items: array<string>) -> (bool | int);  // 联合类型
            fn setCallback(cb: callback(success: bool)) -> void;
        }

        // 类定义
        class ConsoleLogger {
            // 属性
            property level: LogLevel;
            readonly property initialized: bool;

            // 方法
            fn log(message: string) -> void;
            fn setLevel(newLevel: LogLevel) -> void;

            // 构造函数
            ConsoleLogger(initialLevel: LogLevel);
        }

        // 全局函数
        fn createLogger(name: string) -> ConsoleLogger;
        fn createLoggerWithOptions(name: string, options: LogEntry?) -> ConsoleLogger;
        "#;

        match parse_idl(ridl) {
            Ok(items) => {
                // 验证解析结果
                assert_eq!(items.len(), 7); // using + enum + struct + interface + class + 2 global functions

                // 检查using定义
                if let IDLItem::Using(using_def) = &items[0] {
                    assert_eq!(using_def.name, "StringMap");
                    if let Type::Map(key_type, value_type) = &using_def.alias_type {
                        assert_eq!(**key_type, Type::String);
                        assert_eq!(**value_type, Type::String);
                    } else {
                        panic!("Expected map type for StringMap alias");
                    }
                } else {
                    panic!("Expected using definition");
                }

                // 检查枚举
                if let IDLItem::Enum(enum_def) = &items[1] {
                    assert_eq!(enum_def.name, "LogLevel");
                    assert_eq!(enum_def.values.len(), 4);
                    assert!(enum_def.values.iter().any(|v| v.name == "DEBUG"));
                    assert!(enum_def.values.iter().any(|v| v.name == "INFO"));
                    assert!(enum_def.values.iter().any(|v| v.name == "WARN"));
                    assert!(enum_def.values.iter().any(|v| v.name == "ERROR"));
                } else {
                    panic!("Expected enum definition");
                }

                // 检查结构体
                if let IDLItem::Struct(struct_def) = &items[2] {
                    assert_eq!(struct_def.name, "LogEntry");
                    assert_eq!(struct_def.fields.len(), 6);

                    // 检查各字段类型
                    let level_field = struct_def
                        .fields
                        .iter()
                        .find(|f| f.name == "level")
                        .unwrap();
                    assert_eq!(level_field.field_type, Type::Custom("LogLevel".to_string()));

                    let metadata_field = struct_def
                        .fields
                        .iter()
                        .find(|f| f.name == "metadata")
                        .unwrap();
                    if let Type::Map(key_type, value_type) = &metadata_field.field_type {
                        assert_eq!(**key_type, Type::String);
                        assert_eq!(**value_type, Type::String);
                    } else {
                        panic!("Expected map type for metadata field");
                    }

                    let tags_field = struct_def.fields.iter().find(|f| f.name == "tags").unwrap();
                    if let Type::Array(inner_type) = &tags_field.field_type {
                        assert_eq!(**inner_type, Type::String);
                    } else {
                        panic!("Expected array type for tags field");
                    }

                    let callback_field = struct_def
                        .fields
                        .iter()
                        .find(|f| f.name == "callback_func")
                        .unwrap();
                    if let Type::CallbackWithParams(params) = &callback_field.field_type {
                        assert_eq!(params.len(), 2);
                        assert_eq!(params[0].name, "success");
                        assert_eq!(params[0].param_type, Type::Bool);
                        assert_eq!(params[1].name, "result");
                        assert_eq!(params[1].param_type, Type::String);
                    } else {
                        panic!("Expected callback type for callback field");
                    }
                } else {
                    panic!("Expected struct definition");
                }

                // 检查接口
                if let IDLItem::Interface(interface) = &items[3] {
                    assert_eq!(interface.name, "Console");
                    assert_eq!(interface.methods.len(), 5);

                    // 检查模块声明 - 现在应该存在
                    if let Some(module) = &interface.module {
                        assert_eq!(module.module_path, "std.console");
                        assert_eq!(module.version.as_ref().unwrap(), "1.0");
                    } else {
                        panic!("Expected module declaration on interface");
                    }

                    // 检查log方法
                    let log_method = interface.methods.iter().find(|m| m.name == "log").unwrap();
                    assert_eq!(log_method.params.len(), 1);
                    assert_eq!(log_method.params[0].name, "message");
                    assert_eq!(log_method.params[0].param_type, Type::String);
                    assert_eq!(log_method.return_type, Type::Void);

                    // 检查error方法（可空类型参数）
                    let error_method = interface
                        .methods
                        .iter()
                        .find(|m| m.name == "error")
                        .unwrap();
                    if let Type::Optional(inner_type) = &error_method.params[0].param_type {
                        assert_eq!(**inner_type, Type::String);
                    } else {
                        panic!("Expected optional type for error method parameter");
                    }

                    // 检查processMultiple方法（联合类型返回值）
                    let process_method = interface
                        .methods
                        .iter()
                        .find(|m| m.name == "processMultiple")
                        .unwrap();
                    let return_type = match &process_method.return_type {
                        Type::Group(inner) => &**inner, // 如果是分组的，解包
                        other => other,                 // 否则直接使用
                    };
                    if let Type::Union(types) = return_type {
                        assert_eq!(types.len(), 2);
                        assert!(types.iter().any(|t| matches!(t, Type::Bool)));
                        assert!(types.iter().any(|t| matches!(t, Type::Int)));
                    } else {
                        panic!(
                            "Expected union return type for processMultiple method, got {:?}",
                            process_method.return_type
                        );
                    }
                } else {
                    panic!("Expected interface definition");
                }

                // 检查类
                if let IDLItem::Class(class) = &items[4] {
                    assert_eq!(class.name, "ConsoleLogger");
                    assert_eq!(class.properties.len(), 2);
                    assert_eq!(class.methods.len(), 2);
                    assert!(class.constructor.is_some());

                    // 检查属性
                    let level_prop = class.properties.iter().find(|p| p.name == "level").unwrap();
                    assert_eq!(
                        level_prop.property_type,
                        Type::Custom("LogLevel".to_string())
                    );
                    assert!(level_prop.modifiers.contains(&PropertyModifier::ReadWrite));

                    let init_prop = class
                        .properties
                        .iter()
                        .find(|p| p.name == "initialized")
                        .unwrap();
                    assert_eq!(init_prop.property_type, Type::Bool);
                    assert!(init_prop.modifiers.contains(&PropertyModifier::ReadOnly));
                } else {
                    panic!("Expected class definition");
                }

                // 检查第一个全局函数
                if let IDLItem::Function(global_fn) = &items[5] {
                    assert_eq!(global_fn.name, "createLogger");
                    assert_eq!(global_fn.params.len(), 1);
                    assert_eq!(global_fn.params[0].name, "name");
                    assert_eq!(global_fn.params[0].param_type, Type::String);
                    assert_eq!(
                        global_fn.return_type,
                        Type::Custom("ConsoleLogger".to_string())
                    );
                } else {
                    panic!("Expected function definition");
                }

                // 检查第二个全局函数
                if let IDLItem::Function(global_fn) = &items[6] {
                    assert_eq!(global_fn.name, "createLoggerWithOptions");
                    assert_eq!(global_fn.params.len(), 2);
                    assert_eq!(global_fn.params[0].name, "name");
                    assert_eq!(global_fn.params[0].param_type, Type::String);
                    assert_eq!(global_fn.params[1].name, "options");
                    if let Type::Optional(inner_type) = &global_fn.params[1].param_type {
                        assert_eq!(**inner_type, Type::Custom("LogEntry".to_string()));
                    } else {
                        panic!("Expected optional LogEntry for options parameter");
                    }
                    assert_eq!(
                        global_fn.return_type,
                        Type::Custom("ConsoleLogger".to_string())
                    );
                } else {
                    panic!("Expected second function definition");
                }
            }
            Err(e) => panic!("Parsing comprehensive RIDL syntax failed with error: {}", e),
        }
    }

    // 基础语法测试
    #[test]
    fn test_identifier() {
        let result = IDLParser::parse(Rule::identifier, "validIdentifier123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_literal() {
        let result = IDLParser::parse(Rule::string_literal, "\"hello world\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_integer_literal() {
        let result = IDLParser::parse(Rule::integer_literal, "12345");
        assert!(result.is_ok());
    }

    #[test]
    fn test_float_literal() {
        let result = IDLParser::parse(Rule::float_literal, "12.34");
        assert!(result.is_ok());
    }

    // 复杂类型测试
    #[test]
    fn test_nullable_type() {
        let result = IDLParser::parse(Rule::r#type, "string?");
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_type() {
        let result = IDLParser::parse(Rule::r#type, "string | int | bool");
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_type() {
        let result = IDLParser::parse(Rule::r#type, "array<string>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_map_type() {
        let result = IDLParser::parse(Rule::r#type, "map<string, int>");
        assert!(result.is_ok());
    }

    #[test]
    fn test_group_type() {
        let result = IDLParser::parse(Rule::r#type, "(Person | LogEntry | string)");
        assert!(result.is_ok());
    }

    // 接口定义测试
    #[test]
    fn test_interface_definition() {
        let input = r#"
        interface TestInterface {
            fn getValue() -> int;
            fn process(input: string);
            fn optionalParam(name: string?);
        }
        "#;

        let result = IDLParser::parse(Rule::interface_def, input);
        assert!(result.is_ok());
    }

    // 类定义测试
    #[test]
    fn test_class_definition() {
        let input = r#"
        class TestClass {
            var name: string = "";
            var age: int = 0;
            readonly property enabled: bool;
            TestClass(name: string, age: int);
            fn getName() -> string;
            fn setAge(age: int) -> void;
        }
        "#;

        let result = IDLParser::parse(Rule::class_def, input);
        assert!(result.is_ok());
    }

    // 枚举定义测试
    #[test]
    fn test_enum_definition() {
        let input = r#"
        enum TestEnum {
            VALUE1 = 0,
            VALUE2 = 1,
            VALUE3 = 2
        }
        "#;

        let result = IDLParser::parse(Rule::enum_def, input);
        assert!(result.is_ok());
    }

    // 结构体定义测试
    #[test]
    fn test_struct_definition() {
        let input = r#"
        json struct TestStruct {
            field1: string;
            field2: int?;
            field3: array<string>;
        }
        "#;

        let result = IDLParser::parse(Rule::struct_def, input);
        assert!(result.is_ok());
    }

    // 使用不同序列化格式的结构体
    #[test]
    fn test_msgpack_struct_definition() {
        let input = r#"
        msgpack struct TestStruct {
            field1: string;
            field2: int;
        }
        "#;

        let result = IDLParser::parse(Rule::struct_def, input);
        assert!(result.is_ok());
    }

    // 回调定义测试
    #[test]
    fn test_callback_definition() {
        let input = r#"
        callback ProcessCallback(result: string | object, success: bool);
        "#;

        let result = IDLParser::parse(Rule::callback_def, input);
        assert!(result.is_ok());
    }

    // 函数定义测试
    #[test]
    fn test_function_definition() {
        let input = r#"
        fn add(a: int, b: int) -> int;
        "#;

        let result = IDLParser::parse(Rule::global_function, input);
        assert!(result.is_ok());
    }

    // using定义测试
    #[test]
    fn test_using_definition() {
        let input = r#"
        using UserId = int;
        "#;

        let result = IDLParser::parse(Rule::using_def, input);
        assert!(result.is_ok());
    }

    // import语句测试
    #[test]
    fn test_import_definition() {
        let input = r#"
        import NetworkPacket from "Packet.proto";
        "#;

        let result = IDLParser::parse(Rule::import_stmt, input);
        assert!(result.is_ok());
    }

    // 完整RIDL文件测试
    #[test]
    fn test_complete_ridl_file() {
        let input = r#"
        // 完整的RIDL文件示例
        using UserId = int;
        import NetworkPacket from "Packet.proto";
        
        json struct Person {
            name: string;
            age: int;
            email: string?;
        }
        
        interface UserService {
            fn getUser(id: UserId) -> Person?;
            fn processUsers(users: array<Person>, cb: callback(success: bool));
        }
        
        class UserProcessor {
            var cache: any = null;
            UserProcessor();
            fn processUser(user: Person) -> bool;
        }
        
        enum Status {
            PENDING = 0,
            PROCESSING = 1,
            COMPLETED = 2
        }
        
        callback ResultCallback(success: bool, result: string?);
        
        fn setTimeout(cb: callback(success: bool), delay: int);
        "#;

        let result = IDLParser::parse(Rule::idl, input);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
    }

    // 模块化定义测试
    #[test]
    fn test_module_definition() {
        let input = r#"
        module system.network@1.0
        interface Network {
            fn getStatus() -> string;
        }
        "#;

        let result = IDLParser::parse(Rule::idl, input);
        assert!(result.is_ok());
    }

    // singleton定义测试
    #[test]
    fn test_singleton_definition() {
        let input = r#"
        singleton console {
            fn log(message: string);
            fn error(message: string);
            readonly property enabled: bool;
        }
        "#;

        let result = IDLParser::parse(Rule::singleton_def, input);
        assert!(result.is_ok());
    }

    // 复杂联合类型测试
    #[test]
    fn test_complex_union_type() {
        let result = IDLParser::parse(
            Rule::r#type,
            "string | int | array<string> | map<string, int> | Person",
        );
        assert!(result.is_ok());
    }

    // 复杂可空类型测试
    #[test]
    fn test_complex_nullable_type() {
        let result = IDLParser::parse(Rule::r#type, "(string | int)?");
        assert!(result.is_ok());
    }

    // 错误用例测试
    #[test]
    fn test_invalid_interface_missing_brace() {
        let input = r#"interface TestInterface { fn getValue() -> int; "#; // 缺少闭合大括号
        let result = IDLParser::parse(Rule::interface_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_type_definition_array() {
        let input = r#"array<"#; // 不完整的数组类型定义
        let result = IDLParser::parse(Rule::r#type, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_type_definition_map() {
        let input = r#"map<string"#; // 不完整的映射类型定义
        let result = IDLParser::parse(Rule::r#type, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_enum_definition() {
        let input = r#"enum TestEnum { VALUE1 = 0, "#; // 缺少闭合大括号
        let result = IDLParser::parse(Rule::enum_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_struct_definition() {
        let input = r#"struct TestStruct { field1: string; "#; // 缺少闭合大括号
        let result = IDLParser::parse(Rule::struct_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_function_definition() {
        let input = r#"fn add(a: int, "#; // 不完整的函数定义
        let result = IDLParser::parse(Rule::global_function, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_callback_definition() {
        let input = r#"callback ProcessCallback("#; // 不完整的回调定义
        let result = IDLParser::parse(Rule::callback_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_using_definition() {
        let input = r#"using UserId "#; // 不完整的using定义
        let result = IDLParser::parse(Rule::using_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_import_definition() {
        let input = r#"import NetworkPacket from "#; // 不完整的import定义
        let result = IDLParser::parse(Rule::import_stmt, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_class_definition() {
        let input = r#"class TestClass { name: string "#; // 缺少分号
        let result = IDLParser::parse(Rule::class_def, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_singleton_definition() {
        let input = r#"singleton console { fn log(message: string) "#; // 缺少分号
        let result = IDLParser::parse(Rule::singleton_def, input);
        assert!(result.is_err());
    }

    // 修复错误用例：使用更现实的无效语法测试
    #[test]
    fn test_invalid_complete_ridl_file_with_syntax_error() {
        // 一个更复杂的错误用例
        let input = r#"
        interface TestInterface {
            fn getValue() ->;  // 错误：缺少返回类型
        }
        "#;

        let result = IDLParser::parse(Rule::idl, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_literal() {
        let input = r#""hello world"#; // 缺少闭合引号
        let result = IDLParser::parse(Rule::string_literal, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_identifier_with_keyword() {
        let input = r#"interface"#; // 关键字不能作为标识符
        let result = IDLParser::parse(Rule::identifier, input);
        assert!(result.is_err());
    }

    // Module语法的边界条件测试
    #[test]
    fn test_module_with_version() {
        let input = r#"module a@1.0"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_without_version() {
        let input = r#"module b"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_with_version_and_semicolon() {
        let input = r#"module c@2;"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_without_version_with_semicolon() {
        let input = r#"module d;"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_with_complex_path() {
        let input = r#"module system.network.utils@1.5"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_with_complex_path_and_semicolon() {
        let input = r#"module system.network.utils@1.5;"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    // 无效module语法的测试
    #[test]
    fn test_invalid_module_missing_name() {
        let input = r#"module"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_module_with_invalid_version_format() {
        // 测试包含多余版本号的module声明，使用idl规则确保整个输入被解析
        let input = r#"module test@1.0.2.5
interface Test {}"#; // 版本格式错误，包含过多的版本号部分
        let result = IDLParser::parse(Rule::idl, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_module_with_three_part_version() {
        // 测试三部分版本号（应该失败）
        let input = r#"module test@1.2.3"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        // 这种情况下，pest会解析 "1.2" 部分，但因为是module_decl规则，只解析到能匹配的部分
        // 所以这个测试可能仍然通过，因此我们使用idl规则来测试
        assert!(result.is_ok());
    }

    #[test]
    fn test_module_with_three_part_version_with_idl_rule() {
        // 使用idl规则测试三部分版本号（应该失败）
        let input = r#"module test@1.2.3
interface Test {}"#;
        let result = IDLParser::parse(Rule::idl, input);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_module_with_two_part_version() {
        // 测试有效的双部分版本号
        let input = r#"module test@1.2"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_module_with_one_part_version() {
        // 测试有效的单部分版本号
        let input = r#"module test@1"#;
        let result = IDLParser::parse(Rule::module_decl, input);
        assert!(result.is_ok());
    }
}
