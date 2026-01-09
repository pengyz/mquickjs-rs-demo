# RIDL (Rust Interface Description Language) 词法和文法规范

## 1. 概述

RIDL (Rust Interface Description Language) 是一种用于定义 Rust 与 JavaScript 之间接口的接口描述语言。它允许开发者定义接口、类、枚举、结构体等类型，并自动生成相应的绑定代码。

## 2. 词法规范 (Lexical Specification)

### 2.1 关键字 (Keywords)

```
interface, class, enum, struct, const, readonly, property, callback, array, map,
true, false, fn, import, as, from, using, module, singleton
```

### 2.2 标识符 (Identifiers)

```
identifier ::= (letter | "_") (letter | digit | "_")*
letter ::= [a-zA-Z]
digit ::= [0-9]
```

### 2.3 字面量 (Literals)

#### 2.3.1 字符串字面量
```
string_literal ::= "\"" (char | escape_sequence)* "\""
char ::= any character except "\"", "\""
escape_sequence ::= "\" ( "n" | "t" | "r" | "\"" | "'" | "\" )
```

#### 2.3.2 整数字面量
```
integer_literal ::= digit+
```

#### 2.3.3 浮点数字面量
```
float_literal ::= digit+ "." digit+
```

#### 2.3.4 布尔字面量
```
bool_literal ::= "true" | "false"
```

### 2.4 注释 (Comments)

```
// 单行注释
/* 多行注释 */
```

### 2.5 操作符 (Operators)

```
->    (函数返回类型指示符)
|     (联合类型分隔符)
?     (可空类型指示符)
< >   (泛型参数分隔符)
,     (参数分隔符)
:     (类型声明分隔符)
=     (赋值操作符)
;     (语句结束符)
@     (版本分隔符)
```

## 3. 文法规范 (Grammar Specification)

```
// RIDL grammar definition for pest parser
whitespace = _{ (" " | "\t" | "\n" | "\r")+ }

// Comments
comment = _{ "/*" ~ (!"*/" ~ ANY)* ~ "*/" | "//" ~ (!"\n" ~ ANY)* ~ "\n" }

// Keywords - separate lexical rule for keywords
keyword = _{ 
    "interface" | "class" | "enum" | "struct" | "const" | "readonly" | "property" | 
    "callback" | "array" | "map" | "true" | "false" | "fn" | "import" | "as" | 
    "from" | "using" | "module" | "singleton"
}

// Identifier - must not match keywords
identifier = @{ !keyword ~ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }

// Main entry point
idl = { SOI ~ (WS? ~ module_decl ~ WS)? ~ (WS? ~ definition ~ WS?)* ~ EOI }
definition = { interface_def | class_def | enum_def | struct_def | global_function | callback_def | using_def | import_stmt | singleton_def }
module_decl = { "module" ~ WS ~ module_path ~ ("@" ~ WS ~ version)? }
module_path = { identifier ~ ("." ~ identifier)* }
version = { ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }  // 版本号格式：主版本号.次版本号（可选）

interface_def = { WS? ~ "interface" ~ WS ~ identifier ~ WS ~ "{" ~ (WS ~ method_def ~ WS ~ ";")* ~ WS ~ "}" }
class_def = { WS? ~ "class" ~ WS ~ identifier ~ WS ~ "{" ~ (WS ~ class_member ~ WS ~ ";")* ~ WS ~ "}" }
enum_def = { WS? ~ "enum" ~ WS ~ identifier ~ WS ~ "{" ~ WS? ~ enum_value ~ (WS ~ "," ~ WS ~ enum_value)* ~ (WS ~ ",")? ~ WS? ~ "}" }

// Using definition for type aliases
using_def = { WS? ~ "using" ~ WS ~ identifier ~ WS ~ "=" ~ WS ~ type ~ WS ~ ";" }

// Import statement
import_stmt = { WS? ~ "import" ~ WS ~ import_list ~ WS ~ "from" ~ WS ~ string_literal ~ WS ~ ";" }
import_list = { (identifier ~ ("as" ~ WS ~ identifier)? ~ (WS ~ "," ~ WS ~ identifier ~ ("as" ~ WS ~ identifier)?)*)
              | ("*" ~ WS ~ "as" ~ WS ~ identifier) }

// Struct definitions with serialization format
struct_def = { 
    WS? ~ 
    ( 
        ("json" ~ WS ~ "struct" | "msgpack" ~ WS ~ "struct" | "protobuf" ~ WS ~ "struct") | 
        "struct" 
    ) ~ 
    WS ~ identifier ~ WS ~ "{" ~ (WS ~ field_def ~ WS ~ ";")* ~ WS ~ "}" 
}
global_function = { WS? ~ "fn" ~ WS ~ identifier ~ WS ~ "(" ~ WS ~ (param_list)? ~ WS ~ ")" ~ (WS ~ ("->") ~ WS ~ type)? ~ WS ~ ";" }
callback_def = { WS? ~ "callback" ~ WS ~ identifier ~ WS ~ "(" ~ WS ~ (param_list)? ~ WS ~ ")" ~ WS ~ ";" }

// Singleton definition
singleton_def = { WS? ~ "singleton" ~ WS ~ identifier ~ WS ~ "{" ~ (WS ~ singleton_member ~ WS ~ ";")* ~ WS ~ "}" }
singleton_member = { method_def | readonly_prop | readwrite_prop | normal_prop }

// Class members
class_member = { const_member | readonly_prop | readwrite_prop | normal_prop | method_def | constructor }
const_member = { "const" ~ WS ~ identifier ~ WS ~ ":" ~ WS ~ type ~ WS ~ "=" ~ WS ~ literal }
readonly_prop = { "readonly" ~ WS ~ "property" ~ WS ~ identifier ~ WS ~ ":" ~ WS ~ type }
readwrite_prop = { "property" ~ WS ~ identifier ~ WS ~ ":" ~ WS ~ type }
// New property type without special keyword
normal_prop = { identifier ~ WS ~ ":" ~ WS ~ type }
// Method definition: support fn name(params) -> type format, return type is optional
method_def = { "fn" ~ WS ~ identifier ~ WS ~ "(" ~ WS ~ (param_list)? ~ WS ~ ")" ~ (WS ~ ("->") ~ WS ~ type)? }
constructor = { identifier ~ WS ~ "(" ~ WS ~ (param_list)? ~ WS ~ ")" }

// Enum definition - fixed to handle comma properly
enum_value = { identifier ~ (WS ~ "=" ~ WS ~ integer)? }

// Field definition
field_def = { identifier ~ WS ~ ":" ~ WS ~ type }

// Type definitions - using precedence for handling left recursion
type = { union_type | nullable_type | primary_type }
// We need to make sure callback_type comes before basic_type since both could match "callback"
primary_type = { array_type | map_type | custom_type | callback_type | group_type | basic_type }
basic_type = { "bool" | "int" | "float" | "double" | "string" | "void" | "object" | "null" | "any" }
array_type = { "array" ~ WS ~ "<" ~ WS ~ type ~ WS ~ ">" }
map_type = { "map" ~ WS ~ "<" ~ WS ~ type ~ WS ~ "," ~ WS ~ type ~ WS ~ ">" }
union_type = { primary_type ~ (WS ~ "|" ~ WS ~ primary_type)+ }
// 修改nullable_type，防止嵌套，并使用负前瞻防止重复的?符号
nullable_type = { (basic_type | array_type | map_type | custom_type | callback_type | group_type) ~ (WS? ~ "?") ~ (!"?") }
custom_type = { identifier }
callback_type = { "callback" ~ WS ~ identifier? ~ WS ~ "(" ~ WS ~ (param_list)? ~ WS ~ ")" }
group_type = { "(" ~ WS ~ type ~ WS ~ ")" }

// Parameter list - correctly handling empty parameter list
param_list = { param ~ (WS ~ "," ~ WS ~ param)* }
param = { identifier ~ WS ~ ":" ~ WS ~ type }

// Literals
literal = { string_literal | integer_literal | float_literal | bool_literal }
string_literal = { "\"" ~ (!"\"" ~ ANY)* ~ "\"" }
integer_literal = { ASCII_DIGIT+ }
float_literal = { ASCII_DIGIT+ ~ "." ~ ASCII_DIGIT+ }
bool_literal = { "true" | "false" }

// Whitespace handling rule
WS = _{ (whitespace | comment)* }

// Integer for enum values
integer = @{ ASCII_DIGIT+ }
```

## Module Declaration

The `module_decl` rule defines how modules are declared in RIDL files:
- Starts with the `module` keyword
- Followed by a module path with optional version
- **Note: A semicolon after the module declaration is NOT required (unlike other definitions)**
- Module path uses dot notation for hierarchical naming
- Version is optional and specified with `@` followed by version numbers

Example:
```
module system.network@1.0
interface Network {
    fn getStatus() -> string;
}
```

## Definitions

RIDL supports several types of definitions:
- Interface definitions
- Class definitions
- Enum definitions
- Struct definitions (with optional serialization format)
- Global functions
- Callback definitions
- Using aliases
- Import statements
- Singleton definitions

Each definition can optionally be preceded by a module declaration.

## Serialization Formats

Struct definitions support optional serialization format prefixes:
- `json struct` for JSON serialization
- `msgpack struct` for MessagePack serialization
- `protobuf struct` for Protocol Buffers serialization

If no format is specified, the struct uses the default serialization method.

## Type System

RIDL provides a rich type system:
- Basic types (bool, int, float, string, etc.)
- Nullable types using `?` suffix
- Union types using `|` operator
- Array and map types
- Custom types using identifiers
- Callback types for function references
- Group types using parentheses

## Comments and Whitespace

The grammar supports:
- Line comments using `//`
- Block comments using `/* */`
- Flexible whitespace handling throughout

## 4. 语义约束 (Semantic Constraints)

### 4.1 类型约束

- 所有类型必须在使用前声明
- 回调类型只能用于函数参数和返回类型
- 联合类型至少需要两个类型

### 4.2 命名约束

- 标识符不能与关键字冲突
- 接口、类、枚举、结构体名称在全局作用域内必须唯一

### 4.3 结构约束

- 接口不能包含属性，只能包含方法
- 类可以包含属性和方法
- 构造函数名称必须与类名相同
- 枚举值名称在枚举内必须唯一
- RIDL不支持异常定义，错误处理需要通过返回值或回调实现
- RIDL不支持命名空间（namespace），所有定义在全局作用域中
- singleton关键字仅允许用于全局注册，不允许在模块化注册中使用
- singleton表示全局唯一实例对象，用于定义如console等全局对象

## 5. 示例 (Examples)

### 5.1 模块化定义示例

```
// 全局函数
fn setTimeout(callback: callback, delay: int);

// 全局单例对象
singleton console {
    fn log(message: string) -> void;
    fn error(message: string) -> void;
    fn warn(message: string) -> void;
}

// 模块化接口定义
module system.network@1.0
interface Network {
    fn getStatus() -> string;
    fn connect(url: string) -> bool;
}

module system.deviceinfo@1.0
interface DeviceInfo {
    fn getStatus() -> string;
    fn getBatteryLevel() -> int;
}
```

### 5.2 接口定义示例

```
interface Console {
    fn log(message: string);
    fn error(message: string);
    fn warn(message: string);
    fn debug(message: string);
}
```

### 5.3 类定义示例

```
class Person {
    const MAX_AGE: int = 150;
    readonly property name: string;
    property age: int;
    height: float;
    Person(name: string, age: int);
    fn getName() -> string;
    fn getAge() -> int;
    fn setAge(age: int) -> void;
}
```

### 5.4 结构体定义示例

```
json struct Address {
    street: string;
    city: string;
    country: string;
    postalCode: string?;
}

struct Person {
    name: string;
    age: int;
    address: Address;
    tags: array<string>;
}
```

### 5.5 枚举定义示例

```
enum LogLevel {
    DEBUG = 0,
    INFO = 1,
    WARN = 2,
    ERROR = 3,
}
```

### 5.6 回调定义示例

```
callback ProcessCallback(success: bool, result: string);
callback LogCallback(entry: LogEntry);
```

### 5.7 联合类型示例

```
interface DataProcessor {
    fn processInput(data: string | int | array<string>) -> void;
    fn validateData(input: string) -> (bool | object);
}
```

## 6. 模块化机制 (Modularization Mechanism)

### 6.1 require函数

RIDL支持模块化机制，通过全局`require`函数实现：

```javascript
// 获取模块
var network = require("system.network");
network.getStatus();

// 获取特定版本的模块
var network_v1 = require("system.network@1.0");
```

### 6.2 模块注册规则

- 无`module`声明：全局注册到global对象
  - 函数直接注册到global中
  - 单例对象作为属性注册到global上（如global.console）
- 有`module`声明：通过`require("module.name")`访问

### 6.3 语义约束

- singleton关键字仅允许用于全局注册，不允许在模块化注册中使用
- 语义上，singleton表示全局唯一实例，与模块化命名空间概念冲突
- 模块内的功能应通过接口或类定义，而不是单例对象

## 7. 注意事项 (Notes)

1. 接口方法的void返回类型可以省略，即无返回值的函数可以不声明返回类型
2. 结构体可以指定序列化格式(json, msgpack, protobuf)，默认为json
3. 可空类型使用`?`后缀表示，如`string?`表示可空字符串，对应Rust的`Option<T>`类型
4. 数组类型使用`array<T>`语法表示
5. 映射类型使用`map<K, V>`语法表示
6. 联合类型使用`|`分隔符连接多个类型
7. 分号用于分隔接口方法、类成员和结构体字段定义
8. RIDL不支持异常定义，错误处理需要通过返回值或回调实现
9. RIDL不支持命名空间，所有定义都在全局作用域中，因此需要开发者自己管理命名冲突
10. 模块化通过`module`关键字实现，支持版本号声明
11. 单例对象通过`singleton`关键字定义，只能用于全局注册

## 相关文档

- [RIDL_DESIGN.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/RIDL_DESIGN.md) - RIDL设计文档，提供设计原则和语法设计背景
- [IMPLEMENTATION_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/IMPLEMENTATION_GUIDE.md) - 与 Rust 实现的对应关系和代码生成机制
- [FEATURE_DEVELOPMENT_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/FEATURE_DEVELOPMENT_GUIDE.md) - 如何开发和集成基于RIDL的Feature模块
- [TECH_SELECTION.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/ridl-tool/doc/TECH_SELECTION.md) - ridl-tool的技术选型和实现计划