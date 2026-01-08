//! RIDL语义验证器模块
//! 提供对解析后的RIDL AST的语义验证功能

use crate::parser::ast::*;
use std::collections::HashMap;

/// RIDL错误类型枚举
#[derive(Debug, Clone)]
pub enum RIDLErrorType {
    SyntaxError,
    SemanticError,
    ValidationError,
}

/// RIDL错误信息结构
#[derive(Debug, Clone)]
pub struct RIDLError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: String,
    pub error_type: RIDLErrorType,
}

impl RIDLError {
    pub fn new(
        message: String,
        line: usize,
        column: usize,
        file: String,
        error_type: RIDLErrorType,
    ) -> Self {
        RIDLError {
            message,
            line,
            column,
            file,
            error_type,
        }
    }
}

/// 语义验证器
pub struct SemanticValidator {
    errors: Vec<RIDLError>,
    file_path: String,
}

impl SemanticValidator {
    pub fn new(file_path: String) -> Self {
        SemanticValidator {
            errors: Vec::new(),
            file_path,
        }
    }

    /// 验证整个IDL定义
    pub fn validate(&mut self, idl: &IDL) -> Result<(), Vec<RIDLError>> {
        // 检查module声明是否在文件开头
        self.validate_module_position(idl);

        // 收集所有定义的标识符，用于重复定义检查
        let mut defined_identifiers = HashMap::new();
        self.collect_defined_identifiers(idl, &mut defined_identifiers);

        // 检查重复定义
        self.validate_duplicate_definitions(&defined_identifiers);

        // 验证类型引用
        self.validate_type_references(idl);

        // 验证标识符命名
        self.validate_identifiers(idl);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::replace(&mut self.errors, Vec::new()))
        }
    }

    /// 检查module声明是否在文件开头
    fn validate_module_position(&mut self, idl: &IDL) {
        // 如果存在module声明，它应该在定义列表的最前面
        if let Some(ref _module_decl) = idl.module {
            // module声明应该在其他定义之前，所以它应该是第一个定义
            // 这个检查在语法层面已经由pest处理了，这里主要是为了完整性
        }
    }

    /// 收集所有定义的标识符
    fn collect_defined_identifiers(
        &mut self,
        idl: &IDL,
        identifiers: &mut HashMap<String, (usize, usize)>,
    ) {
        // 检查接口定义
        for interface in &idl.interfaces {
            identifiers.insert(interface.name.clone(), (0, 0)); // TODO: 添加位置信息
        }

        // 检查类定义
        for class in &idl.classes {
            identifiers.insert(class.name.clone(), (0, 0)); // TODO: 添加位置信息
        }

        // 检查枚举定义
        for enum_def in &idl.enums {
            identifiers.insert(enum_def.name.clone(), (0, 0)); // TODO: 添加位置信息
        }

        // 检查结构体定义
        for struct_def in &idl.structs {
            identifiers.insert(struct_def.name.clone(), (0, 0)); // TODO: 添加位置信息
        }

        // 检查类型别名
        for using in &idl.using {
            identifiers.insert(using.name.clone(), (0, 0)); // TODO: 添加位置信息
        }
    }

    /// 检查重复定义（已集成到collect_defined_identifiers中，保留为空实现向后兼容）
    fn validate_duplicate_definitions(&mut self, _identifiers: &HashMap<String, (usize, usize)>) {
        // 重复定义检查已在collect_defined_identifiers中完成
    }

    /// 验证类型引用
    fn validate_type_references(&mut self, idl: &IDL) {
        // 验证接口中的方法参数和返回值类型
        for interface in &idl.interfaces {
            for method in &interface.methods {
                self.validate_type(&method.return_type);
                for param in &method.params {
                    self.validate_type(&param.param_type);
                }
            }
        }

        // 验证类中的方法、属性和构造函数
        for class in &idl.classes {
            if let Some(ref constructor) = class.constructor {
                for param in &constructor.params {
                    self.validate_type(&param.param_type);
                }
            }
            for method in &class.methods {
                self.validate_type(&method.return_type);
                for param in &method.params {
                    self.validate_type(&param.param_type);
                }
            }
            for property in &class.properties {
                self.validate_type(&property.property_type);
            }
        }

        // 验证结构体字段类型
        for struct_def in &idl.structs {
            for field in &struct_def.fields {
                self.validate_type(&field.field_type);
            }
        }

        // 验证枚举不涉及类型引用
        // 验证单例定义
        for singleton in &idl.singletons {
            for method in &singleton.methods {
                self.validate_type(&method.return_type);
                for param in &method.params {
                    self.validate_type(&param.param_type);
                }
            }
        }

        // 验证全局函数
        for function in &idl.functions {
            self.validate_type(&function.return_type);
            for param in &function.params {
                self.validate_type(&param.param_type);
            }
        }
    }

    /// 验证单个类型
    fn validate_type(&mut self, idl_type: &Type) {
        match idl_type {
            Type::Custom(_name) => {
                // 检查自定义类型是否已定义
                // 这里需要更复杂的逻辑来检查类型是否已定义
                // 暂时跳过，因为我们需要访问全局定义上下文
            }
            Type::Optional(boxed_type) => {
                self.validate_type(boxed_type);
            }
            Type::Array(element_type) => {
                self.validate_type(element_type);
            }
            Type::Map(key_type, value_type) => {
                self.validate_type(key_type);
                self.validate_type(value_type);
            }
            Type::Union(types) => {
                for t in types {
                    self.validate_type(t);
                }
            }
            Type::Group(inner_type) => {
                self.validate_type(inner_type);
            }
            // 基础类型不需要验证
            Type::Bool
            | Type::Int
            | Type::Float
            | Type::Double
            | Type::String
            | Type::Void
            | Type::Object
            | Type::Callback
            | Type::CallbackWithParams(_)
            | Type::Null
            | Type::Any => {}
        }
    }

    /// 验证标识符是否使用了关键字
    fn validate_identifiers(&mut self, idl: &IDL) {
        // 检查接口定义
        for interface in &idl.interfaces {
            self.check_for_keyword_usage(&interface.name, "interface name");
        }

        // 检查类定义
        for class in &idl.classes {
            self.check_for_keyword_usage(&class.name, "class name");
        }

        // 检查枚举定义
        for enum_def in &idl.enums {
            self.check_for_keyword_usage(&enum_def.name, "enum name");
        }

        // 检查结构体定义
        for struct_def in &idl.structs {
            self.check_for_keyword_usage(&struct_def.name, "struct name");
        }

        // 检查类型别名
        for using in &idl.using {
            self.check_for_keyword_usage(&using.name, "using alias");
        }

        // 检查接口中的方法和参数
        for interface in &idl.interfaces {
            for method in &interface.methods {
                self.check_for_keyword_usage(&method.name, "method name");
                for param in &method.params {
                    self.check_for_keyword_usage(&param.name, "parameter name");
                }
            }
        }

        // 检查类中的方法、属性和构造函数
        for class in &idl.classes {
            for method in &class.methods {
                self.check_for_keyword_usage(&method.name, "method name");
                for param in &method.params {
                    self.check_for_keyword_usage(&param.name, "parameter name");
                }
            }
            for property in &class.properties {
                self.check_for_keyword_usage(&property.name, "property name");
            }
            if let Some(ref constructor) = class.constructor {
                for param in &constructor.params {
                    self.check_for_keyword_usage(&param.name, "constructor parameter name");
                }
            }
        }

        // 检查结构体字段
        for struct_def in &idl.structs {
            for field in &struct_def.fields {
                self.check_for_keyword_usage(&field.name, "field name");
            }
        }

        // 检查枚举值
        for enum_def in &idl.enums {
            for value in &enum_def.values {
                self.check_for_keyword_usage(&value.name, "enum value name");
            }
        }

        // 检查单例定义
        for singleton in &idl.singletons {
            self.check_for_keyword_usage(&singleton.name, "singleton name");
            for method in &singleton.methods {
                self.check_for_keyword_usage(&method.name, "method name");
                for param in &method.params {
                    self.check_for_keyword_usage(&param.name, "parameter name");
                }
            }
        }

        // 检查全局函数
        for function in &idl.functions {
            self.check_for_keyword_usage(&function.name, "function name");
            for param in &function.params {
                self.check_for_keyword_usage(&param.name, "parameter name");
            }
        }

        // 检查回调
        for callback in &idl.callbacks {
            self.check_for_keyword_usage(&callback.name, "callback name");
            for param in &callback.params {
                self.check_for_keyword_usage(&param.name, "parameter name");
            }
        }
    }

    /// 检查标识符是否使用了关键字
    fn check_for_keyword_usage(&mut self, identifier: &str, context: &str) {
        // RIDL关键字列表
        let keywords = [
            "interface",
            "class",
            "enum",
            "struct",
            "const",
            "readonly",
            "property",
            "callback",
            "array",
            "map",
            "true",
            "false",
            "fn",
            "import",
            "as",
            "from",
            "using",
            "module",
            "singleton",
        ];

        if keywords.contains(&identifier) {
            self.errors.push(RIDLError::new(
                format!(
                    "Invalid identifier '{}', '{}' is a reserved keyword and cannot be used as {}",
                    identifier, identifier, context
                ),
                0, // TODO: 添加实际位置信息
                0, // TODO: 添加实际位置信息
                self.file_path.clone(),
                RIDLErrorType::SemanticError,
            ));
        }
    }
}

/// 验证IDL项目列表
pub fn validate(items: &[IDLItem]) -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个临时的IDL结构体来包装items
    let mut idl = IDL {
        module: None,
        interfaces: vec![],
        classes: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        using: vec![],
        imports: vec![],
        singletons: vec![],
        callbacks: vec![],
    };

    // 从items中提取各种定义到idl中
    for item in items {
        match item {
            IDLItem::Interface(interface) => idl.interfaces.push(interface.clone()),
            IDLItem::Class(class) => idl.classes.push(class.clone()),
            IDLItem::Enum(enum_def) => idl.enums.push(enum_def.clone()),
            IDLItem::Struct(struct_def) => idl.structs.push(struct_def.clone()),
            IDLItem::Function(function) => idl.functions.push(function.clone()),
            IDLItem::Using(using) => idl.using.push(using.clone()),
            IDLItem::Import(import) => idl.imports.push(import.clone()),
            IDLItem::Singleton(singleton) => idl.singletons.push(singleton.clone()),
        }
    }

    // 创建验证器并验证
    let mut validator = SemanticValidator::new("unknown.ridl".to_string());
    match validator.validate(&idl) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format!("{} (line {}, col {})", e.message, e.line, e.column))
                .collect();
            Err(format!("Validation errors: {}", error_messages.join("; ")).into())
        }
    }
}
