use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleDeclaration {
    pub module_path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IDL {
    pub module: Option<ModuleDeclaration>,
    pub interfaces: Vec<Interface>,
    pub classes: Vec<Class>,
    pub enums: Vec<Enum>,
    pub structs: Vec<StructDef>,
    pub functions: Vec<Function>,
    pub using: Vec<Using>,
    pub imports: Vec<Import>,
    pub singletons: Vec<Singleton>,
    pub callbacks: Vec<Function>, // 为简单起见，将回调作为函数处理
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IDLItem {
    Interface(Interface),
    Class(Class),
    Enum(Enum),
    Struct(StructDef),
    Function(Function),
    Using(Using),
    Import(Import),
    Singleton(Singleton),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Class {
    pub name: String,
    pub constructor: Option<Function>,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub serialization_format: SerializationFormat,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Singleton {
    pub name: String,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Property {
    pub modifiers: Vec<PropertyModifier>,
    pub name: String,
    pub property_type: Type,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PropertyModifier {
    ReadOnly,
    ReadWrite,
    Const,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Param {
    pub name: String,
    pub param_type: Type,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Using {
    pub name: String,
    pub alias_type: Type,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Import {
    pub imports: Vec<ImportItem>,
    pub path: String,
    pub module: Option<ModuleDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SerializationFormat {
    Json,
    MessagePack,
    Protobuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Type {
    Bool,
    Int,
    Float,
    Double,
    String,
    Void,
    Object,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Union(Vec<Type>),
    Optional(Box<Type>),
    Custom(String),
    Callback,
    CallbackWithParams(Vec<Param>),
    Group(Box<Type>),
    Null,
    Any,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Object => write!(f, "object"),
            Type::Array(t) => write!(f, "array<{}>", t),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            Type::Optional(t) => write!(f, "{}?", t),
            Type::Custom(name) => write!(f, "{}", name),
            Type::Callback => write!(f, "callback"),
            Type::CallbackWithParams(params) => {
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.param_type))
                    .collect();
                write!(f, "callback({})", param_strs.join(", "))
            }
            Type::Group(t) => write!(f, "({})", t),
            Type::Null => write!(f, "null"),
            Type::Any => write!(f, "any"),
        }
    }
}
