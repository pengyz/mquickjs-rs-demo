/// IDL项的枚举
#[derive(Debug, Clone)]
pub enum IDLItem {
    Interface(Interface),
    Class(Class),
    Enum(Enum),
    Struct(StructDef),
    GlobalFunction(Function),
}

/// 接口定义
#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// 类定义
#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub constructor: Option<Function>,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}

/// 枚举定义
#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<i32>, // 可选的显式值
}

/// 结构体定义
#[derive(Debug, Clone)]
pub struct StructDef {
    pub serialization_format: SerializationFormat,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub enum SerializationFormat {
    Json,
    MsgPack,
    Protobuf,
}

/// 字段定义
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
    pub optional: bool,
}

/// 属性定义
#[derive(Debug, Clone)]
pub struct Property {
    pub modifiers: Vec<PropertyModifier>,
    pub name: String,
    pub property_type: Type,
    pub default_value: Option<String>, // 仅对const有效
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyModifier {
    Const,
    Readonly,
    ReadWrite,
}

/// 方法定义
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
}

/// 函数定义
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
}

/// 参数定义
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub param_type: Type,
    pub optional: bool,
}

/// 类型定义
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    Int,
    Float,
    String,
    Void,
    Any,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>), // key, value
    Union(Vec<Type>), // 联合类型
    Optional(Box<Type>), // 可选类型
    Custom(String), // 自定义类型
    Callback(String), // 回调类型
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Any => write!(f, "any"),
            Type::Array(t) => write!(f, "array<{}>", t),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            Type::Optional(t) => write!(f, "{}?", t),
            Type::Custom(name) => write!(f, "{}", name),
            Type::Callback(name) => write!(f, "callback<{}>", name),
        }
    }
}