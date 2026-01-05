use serde::{Deserialize, Serialize};

/// IDL项的枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IDLItem {
    Interface(Interface),
    Class(Class),
    Enum(Enum),
    Struct(StructDef),
    Function(Function),
    Using(UsingDef),
    Namespace(Namespace),
    Import(ImportStmt),
    Exception(Exception),
}

/// 接口定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// 类定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub constructor: Option<Function>,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// 枚举定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<i32>, // 可选的显式值
}

/// 结构体定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub serialization_format: SerializationFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SerializationFormat {
    Json,
    MessagePack,
    Protobuf,
}

/// 字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
    pub optional: bool,
}

/// 属性定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub modifiers: Vec<PropertyModifier>,
    pub name: String,
    pub property_type: Type,
    pub default_value: Option<String>, // 仅对const有效
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyModifier {
    ReadOnly,
    Const,
    ReadWrite,
}

/// 方法定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
}

/// 使用定义（类型别名）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsingDef {
    pub name: String,
    pub alias: String,
    pub target: String,
}

/// 命名空间定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub name: String,
    pub items: Vec<IDLItem>,
}

/// 导入语句
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStmt {
    pub module: String,
    pub items: Vec<ImportItem>,
}

/// 导入项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    pub original_name: String,
    pub alias: Option<String>,
}

/// 异常定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exception {
    pub name: String,
    pub fields: Vec<Field>,
}

/// 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub param_type: Type,
    pub optional: bool,
}

/// 类型定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Type {
    Bool,
    Int,
    Float,
    Double,
    String,
    Void,
    Object,
    Function,
    Callback,
    Null,
    Any,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Union(Vec<Type>),
    Optional(Box<Type>),
    Custom(String),
    Group(Box<Type>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Object => write!(f, "object"),
            Type::Function => write!(f, "function"),
            Type::Callback => write!(f, "callback"),
            Type::Null => write!(f, "null"),
            Type::Any => write!(f, "any"),
            Type::Array(t) => write!(f, "array<{}>", t),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            Type::Optional(t) => write!(f, "{}?", t),
            Type::Custom(name) => write!(f, "{}", name),
            Type::Group(t) => write!(f, "({})", t),
        }
    }
}