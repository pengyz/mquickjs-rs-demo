# mquickjs IDL 与 Rust 实现对应关系指南

## 概述

本文档详细描述了 mquickjs IDL 语法如何与 Rust 实现对应，以及代码生成机制。明确区分了工具生成的胶水代码和开发者需要实现的业务逻辑。

## IDL 与 Rust 代码生成机制

### 1. IDL 解析与 AST 生成

IDL 文件首先被解析为抽象语法树 (AST)，然后根据语法结构生成相应的 Rust 代码。

### 2. 代码生成层次结构

```
IDL 定义
  ↓ (解析)
AST (抽象语法树)
  ↓ (代码生成)
1. 胶水代码 (Glue Code) - 工具自动生成
2. 绑定代码 (Bindings) - 工具自动生成
3. 标准库描述代码 (Stdlib Description) - 工具自动生成
4. 用户实现 (User Implementation) - 开发者编写
```

## IDL 语法元素与 Rust 代码映射

### 1. struct 映射

#### IDL 定义
```
json struct Person {
    name: string;
    age: int;
    email: string?;
}
```

#### 生成的 Rust 代码 (胶水代码)
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Person {
    pub name: String,
    pub age: i32,
    pub email: Option<String>,
}

impl FromJs for Person {
    fn from_js_value(js_value: JSValue, ctx: &Context) -> Result<Self, String> {
        let json_str = ctx.get_string(js_value)?;
        serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to deserialize Person: {}", e))
    }
}

impl ToJs for Person {
    fn to_js_value(&self, ctx: &Context) -> Result<JSValue, String> {
        let json_str = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize Person: {}", e))?;
        ctx.create_string(&json_str)
    }
}
```

### 2. interface 映射

#### IDL 定义
```
interface Console {
    void log(string message);
    void error(string message);
}
```

#### 生成的 Rust 代码 (胶水代码)
```rust
pub trait Console {
    fn log(&self, message: String) -> Result<(), String>;
    fn error(&self, message: String) -> Result<(), String>;
}

// 生成的 C 绑定函数
extern "C" fn console_log_js_binding(
    ctx: *mut mquickjs_ffi::JSContext,
    _this: mquickjs_ffi::JSValue,
    argc: mquickjs_ffi::c_int,
    argv: *mut mquickjs_ffi::JSValue
) -> mquickjs_ffi::JSValue {
    // 参数解析胶水代码
    let message = unsafe {
        // 从 JSValue 提取字符串
        extract_string_from_js_value(ctx, *argv.add(0))
    };
    
    // 调用用户实现
    let console_impl = get_console_implementation();
    match console_impl.log(message) {
        Ok(_) => unsafe { mquickjs_ffi::JS_UNDEFINED },
        Err(e) => unsafe { 
            mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, &e))
        },
    }
}

extern "C" fn console_error_js_binding(
    ctx: *mut mquickjs_ffi::JSContext,
    _this: mquickjs_ffi::JSValue,
    argc: mquickjs_ffi::c_int,
    argv: *mut mquickjs_ffi::JSValue
) -> mquickjs_ffi::JSValue {
    // 参数解析胶水代码
    let message = unsafe {
        // 从 JSValue 提取字符串
        extract_string_from_js_value(ctx, *argv.add(0))
    };
    
    // 调用用户实现
    let console_impl = get_console_implementation();
    match console_impl.error(message) {
        Ok(_) => unsafe { mquickjs_ffi::JS_UNDEFINED },
        Err(e) => unsafe { 
            mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, &e))
        },
    }
}
```

#### 生成的标准库描述代码 (mqjs_stdlib 部分)
``c
// 生成的 C 函数声明
JSValue mqjs_console_log(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);
JSValue mqjs_console_error(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);

// 生成的函数定义数组
static const JSCFunctionListEntry mqjs_console_funcs[] = {
    JS_CFUNC_DEF("log", 1, mqjs_console_log ),
    JS_CFUNC_DEF("error", 1, mqjs_console_error ),
};

// 生成的模块定义
static int js_console_init(JSContext *ctx, JSModuleDef *m) {
    JSValue proto, obj;

    /* Console class */
    JS_NewClassID(&js_console_class_id);
    JS_NewClass(JS_GetRuntime(ctx), js_console_class_id, &js_console_class_def);

    proto = JS_NewObject(ctx);
    JS_SetPropertyFunctionList(ctx, proto, mqjs_console_funcs, countof(mqjs_console_funcs));

    obj = JS_NewObjectProto(ctx, proto);
    JS_SetPropertyStr(ctx, obj, "log", JS_NewCFunction(ctx, mqjs_console_log, "log", 1));
    JS_SetPropertyStr(ctx, obj, "error", JS_NewCFunction(ctx, mqjs_console_error, "error", 1));

    JS_SetModuleExport(ctx, m, "console", obj);
    return 0;
}

// 生成的模块定义和初始化
JSModuleDef *js_init_module_std(JSContext *ctx, const char *name) {
    JSModuleDef *m;
    m = JS_NewCModule(ctx, name, js_console_init);
    if (!m) return NULL;
    JS_AddModuleExport(ctx, m, "console");
    return m;
}
```

### 3. class 映射

#### IDL 定义
```
class Person {
    string name;
    int age;
    
    Person(string name, int age);
    string getName();
    int getAge();
    void setAge(int age);
}
```

#### 生成的 Rust 代码 (胶水代码)
```rust
pub struct Person {
    pub name: String,
    pub age: i32,
}

impl Person {
    pub fn new(name: String, age: i32) -> Self {
        Self { name, age }
    }
    
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    
    pub fn get_age(&self) -> i32 {
        self.age
    }
    
    pub fn set_age(&mut self, age: i32) {
        self.age = age;
    }
}

// 生成的 C 绑定函数
extern "C" fn person_constructor(
    ctx: *mut mquickjs_ffi::JSContext,
    _new_target: mquickjs_ffi::JSValue,
    argc: mquickjs_ffi::c_int,
    argv: *mut mquickjs_ffi::JSValue
) -> mquickjs_ffi::JSValue {
    let name = unsafe { extract_string_from_js_value(ctx, *argv.add(0)) };
    let age = unsafe { extract_int32_from_js_value(ctx, *argv.add(1)) };
    
    let person = Person::new(name, age);
    // 将 person 实例存储到 JS 对象中
    // ... 实现细节
    todo!()
}

extern "C" fn person_get_name(
    ctx: *mut mquickjs_ffi::JSContext,
    this_val: mquickjs_ffi::JSValue,
    argc: mquickjs_ffi::c_int,
    argv: *mut mquickjs_ffi::JSValue
) -> mquickjs_ffi::JSValue {
    // 从 this_val 获取 Person 实例
    let person = get_person_from_js_value(this_val);
    let name = person.get_name();
    // 转换为 JSValue
    create_js_string(ctx, &name)
}
```

#### 生成的标准库描述代码 (mqjs_stdlib 部分)
``c
// 生成的 C 函数声明
JSValue mqjs_person_constructor(JSContext *ctx, JSValueConst new_target, int argc, JSValueConst *argv);
JSValue mqjs_person_get_name(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);
JSValue mqjs_person_get_age(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);
JSValue mqjs_person_set_age(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);

// 生成的类定义
static JSClassDef js_person_class_def = {
    "Person",
    .finalizer = js_person_finalizer,
    .gc_mark = js_person_mark,
};

static const JSCFunctionListEntry js_person_proto_funcs[] = {
    JS_CFUNC_DEF("getName", 0, mqjs_person_get_name),
    JS_CFUNC_DEF("getAge", 0, mqjs_person_get_age),
    JS_CFUNC_DEF("setAge", 1, mqjs_person_set_age),
};

static int js_person_init(JSContext *ctx, JSModuleDef *m) {
    JS_NewClassID(&js_person_class_id);
    JS_NewClass(JS_GetRuntime(ctx), js_person_class_id, &js_person_class_def);

    JSValue proto = JS_NewObject(ctx);
    JS_SetPropertyFunctionList(ctx, proto, js_person_proto_funcs, countof(js_person_proto_funcs));

    JSValue ctor = JS_NewCFunction2(ctx, mqjs_person_constructor, "Person", 2, JS_CFUNC_constructor, 0);
    JS_SetConstructor(ctx, ctor, proto);

    if (m) {
        JS_SetModuleExport(ctx, m, "Person", ctor);
    }

    return 0;
}
```

### 4. callback/异步处理映射

#### IDL 定义
```
using ProcessCallback = callback(string | object result, bool success);

interface AsyncProcessor {
    void processAsync(string data, ProcessCallback callback);
}
```

#### 生成的 Rust 代码 (胶水代码)
```rust
pub trait AsyncProcessor {
    fn process_async(&self, data: String, callback: AsyncCallback) -> Result<(), String>;
}

// 异步处理胶水代码
extern "C" fn async_processor_process_async(
    ctx: *mut mquickjs_ffi::JSContext,
    _this: mquickjs_ffi::JSValue,
    argc: mquickjs_ffi::c_int,
    argv: *mut mquickjs_ffi::JSValue
) -> mquickjs_ffi::JSValue {
    // 提取参数
    let data = unsafe { extract_string_from_js_value(ctx, *argv.add(0)) };
    let callback_js_value = unsafe { *argv.add(1) };
    
    // 包装 JS 回调为 Rust 类型
    let callback = AsyncCallback::new(ctx, callback_js_value);
    
    // 调用用户实现
    let processor_impl = get_async_processor_implementation();
    match processor_impl.process_async(data, callback) {
        Ok(_) => unsafe { mquickjs_ffi::JS_UNDEFINED },
        Err(e) => unsafe { 
            mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, &e))
        },
    }
}
```

#### 生成的标准库描述代码 (mqjs_stdlib 部分)
``c
// 生成的 C 函数声明
JSValue mqjs_async_processor_process_async(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv);

// 生成的函数定义数组
static const JSCFunctionListEntry mqjs_async_processor_funcs[] = {
    JS_CFUNC_DEF("processAsync", 2, mqjs_async_processor_process_async),
};

static int js_async_processor_init(JSContext *ctx, JSModuleDef *m) {
    JSValue proto, obj;

    proto = JS_NewObject(ctx);
    JS_SetPropertyFunctionList(ctx, proto, mqjs_async_processor_funcs, countof(mqjs_async_processor_funcs));

    obj = JS_NewObjectProto(ctx, proto);
    JS_SetPropertyStr(ctx, obj, "processAsync", 
        JS_NewCFunction(ctx, mqjs_async_processor_process_async, "processAsync", 2));

    if (m) {
        JS_SetModuleExport(ctx, m, "asyncProcessor", obj);
    }
    return 0;
}
```

## 用户实现部分

开发者需要实现的具体内容：

### 1. 实现接口 trait
```
struct ConsoleImpl;

impl Console for ConsoleImpl {
    fn log(&self, message: String) -> Result<(), String> {
        // 用户自定义的实现逻辑
        println!("LOG: {}", message);
        Ok(())
    }
    
    fn error(&self, message: String) -> Result<(), String> {
        // 用户自定义的实现逻辑
        eprintln!("ERROR: {}", message);
        Ok(())
    }
}
```

### 2. 实现异步接口
```
struct AsyncProcessorImpl;

impl AsyncProcessor for AsyncProcessorImpl {
    fn process_async(&self, data: String, callback: AsyncCallback) -> Result<(), String> {
        // 启动异步任务
        tokio::spawn(async move {
            // 实际的异步处理逻辑
            let result = perform_async_operation(data).await;
            
            // 通过回调返回结果
            match result {
                Ok(value) => callback.call_with_args(value, true),
                Err(e) => callback.call_with_args(e, false),
            }
        });
        
        Ok(())
    }
}
```

## 类型转换处理机制

### 1. 基础类型映射

| IDL 类型 | JS 类型 | Rust 类型 | 转换函数 |
|----------|---------|-----------|----------|
| `bool` | Boolean | `bool` | `js_to_bool` / `bool_to_js` |
| `int` | Number | `i32` | `js_to_int32` / `int32_to_js` |
| `float` | Number | `f64` | `js_to_float64` / `float64_to_js` |
| `string` | String | `String` | `js_to_string` / `string_to_js` |
| `array<T>` | Array | `Vec<T>` | `js_to_vec` / `vec_to_js` |

### 2. 复杂类型转换

- **联合类型**：生成匹配函数，按顺序尝试转换
- **可空类型**：使用 `Option<T>`，映射到 JS 的 `null/undefined`
- **字典类型**：使用 `HashMap<String, V>`，映射到 JS 对象

### 3. 转换辅助函数

```
// 生成的类型转换辅助函数
pub fn convert_js_value_to_rust_type(
    js_value: JSValue,
    ctx: &Context
) -> Result<RustType, String> {
    if js_value.is_string(ctx) {
        // 转换为字符串类型
        ctx.get_string(js_value).map(RustType::String)
    } else if js_value.is_number(ctx) {
        // 转换为数字类型
        ctx.get_number(js_value).map(|n| RustType::Number(n as i32))
    } else {
        Err("Type conversion failed".to_string())
    }
}
```

## 代码生成流程

1. **解析阶段**：IDL 解析器将 IDL 文件转换为 AST
2. **验证阶段**：验证类型定义的正确性
3. **生成阶段**：
   - 生成 Rust 类型定义（struct）
   - 生成 trait 定义（interface/class）
   - 生成 C 绑定函数
   - 生成类型转换函数
   - 生成 mqjs_stdlib 标准库描述代码（C 语言部分）
4. **mqjs_stdlib 构建阶段**：将生成的 C 代码编译为标准库
5. **编译阶段**：将生成的 Rust 代码与用户实现编译成库

## 胶水代码与用户代码分离

- **胶水代码**：IDL 工具自动生成，处理 JS 与 Rust 之间的类型转换、函数调用、错误处理等
- **标准库描述代码**：IDL 工具自动生成 C 代码，用于注册到 mquickjs 标准库
- **用户代码**：开发者编写实际的业务逻辑，实现生成的 trait
- **绑定代码**：将用户实现与 JS 环境连接，包括函数注册、类定义等

这种分离确保了开发者只需关注业务逻辑，而无需处理底层的类型转换和绑定细节。

# RIDL Implementation Guide

## 模块化机制实现方案

### 1. 整体架构

RIDL的模块化机制通过全局`require`函数实现，该函数允许用户通过模块名获取功能对象，避免全局命名冲突。

1. **模块声明**：在RIDL中使用`module system.network@1.0`语法声明模块
2. **对象注册**：模块中的接口、类等被注册为mquickjs中的class，但不提供构造函数
3. **映射表**：维护模块名到ClassID的映射表
4. **require函数**：全局函数，根据模块名查询映射表并创建对象实例

### 2. 模块注册表实现

#### 2.1 独立模块文件生成

jidl-tool需要生成一个独立的模块注册表文件，通常命名为`module_registry.rs`，该文件将在mquickjs-rs编译时被引入：

```
// 生成的模块注册表文件: module_registry.rs
use std::sync::{Mutex, LazyLock};
use std::collections::HashMap;
use mquickjs::{Context, JSValue, Object, Result};

// 模块映射表 - 存储模块名到创建函数的映射
static MODULE_CREATORS: LazyLock<Mutex<HashMap<String, fn(&Context) -> Result<JSValue>>>> = LazyLock::new(|| {
    Mutex::new(HashMap::from([
        ("system.network".to_string(), create_network_module as fn(&Context) -> Result<JSValue>),
        ("system.deviceinfo".to_string(), create_deviceinfo_module as fn(&Context) -> Result<JSValue>),
        ("ui.components".to_string(), create_ui_components_module as fn(&Context) -> Result<JSValue>),
    ]))
});

// 为每个模块生成创建函数
fn create_network_module(ctx: &Context) -> Result<JSValue> {
    let obj = ctx.new_object()?;
    
    // 设置模块方法
    obj.set("getStatus", ctx.new_function("getStatus", network_get_status)?)?;
    obj.set("connect", ctx.new_function("connect", network_connect)?)?;
    
    Ok(obj.into())
}

fn create_deviceinfo_module(ctx: &Context) -> Result<JSValue> {
    let obj = ctx.new_object()?;
    
    // 设置模块方法
    obj.set("getStatus", ctx.new_function("getStatus", deviceinfo_get_status)?)?;
    obj.set("getBatteryLevel", ctx.new_function("getBatteryLevel", deviceinfo_get_battery_level)?)?;
    
    Ok(obj.into())
}

fn create_ui_components_module(ctx: &Context) -> Result<JSValue> {
    let obj = ctx.new_object()?;
    
    // 设置模块方法
    obj.set("createButton", ctx.new_function("createButton", ui_create_button)?)?;
    obj.set("createLabel", ctx.new_function("createLabel", ui_create_label)?)?;
    
    Ok(obj.into())
}

// require函数实现
pub fn js_require(ctx: &Context, _this: JSValue, args: &[JSValue]) -> Result<JSValue> {
    let module_name = args[0].as_string().ok_or("Module name must be a string")?;
    
    let creators = MODULE_CREATORS.lock().map_err(|e| {
        mquickjs::Error::RustError(format!("Failed to acquire lock: {}", e))
    })?;
    
    if let Some(creator_fn) = creators.get(&module_name) {
        creator_fn(ctx)
    } else {
        Err(mquickjs::Error::RustError(format!("Module '{}' not found", module_name)))
    }
}

// 在模块初始化时注册require函数到全局作用域
pub fn register_require_function(ctx: &Context) -> Result<()> {
    ctx.add_global_function("require", js_require)
}
```

#### 2.2 代码生成器实现

jidl-tool需要增强代码生成器，使其能够根据RIDL文件中的模块声明自动生成映射表：

```
// 伪代码：jidl-tool中的模块映射表生成器
impl ModuleRegistryGenerator {
    pub fn generate_module_registry(&self, items: &[IDLItem]) -> String {
        let mut module_creators = Vec::new();
        let mut creator_registrations = Vec::new();
        let mut creator_functions = Vec::new();
        
        for item in items {
            if let Some(ref module_info) = item.module_info {
                let module_name = &module_info.path;
                let module_fn_name = format!("create_{}_module", 
                    module_name.replace(".", "_").replace("-", "_"));
                
                // 添加到映射表
                creator_registrations.push(format!(
                    "        (\"{}\".to_string(), {} as fn(&Context) -> Result<JSValue>),",
                    module_name, module_fn_name
                ));
                
                // 生成创建函数
                let creator_fn = self.generate_module_creator_function(&module_fn_name, item, module_name);
                creator_functions.push(creator_fn);
            }
        }
        
        // 生成完整的模块注册表文件内容
        format!(r#"// 生成的模块注册表文件: module_registry.rs
use std::sync::{{Mutex, LazyLock}};
use std::collections::HashMap;
use mquickjs::{{Context, JSValue, Object, Result}};

// 模块映射表 - 存储模块名到创建函数的映射
static MODULE_CREATORS: LazyLock<Mutex<HashMap<String, fn(&Context) -> Result<JSValue>>>> = LazyLock::new(|| {{
    Mutex::new(HashMap::from([
{}
    ]))
}});

{}

// require函数实现
pub fn js_require(ctx: &Context, _this: JSValue, args: &[JSValue]) -> Result<JSValue> {{
    let module_name = args[0].as_string().ok_or("Module name must be a string")?;
    
    let creators = MODULE_CREATORS.lock().map_err(|e| {{
        mquickjs::Error::RustError(format!("Failed to acquire lock: {{}}", e))
    }})?;
    
    if let Some(creator_fn) = creators.get(&module_name) {{
        creator_fn(ctx)
    }} else {{
        Err(mquickjs::Error::RustError(format!("Module '{{}}' not found", module_name)))
    }}
}}

// 在模块初始化时注册require函数到全局作用域
pub fn register_require_function(ctx: &Context) -> Result<()> {{
    ctx.add_global_function("require", js_require)
}}
"#,
            creator_registrations.join("        "),
            creator_functions.join("\n\n")
        )
    }
    
    fn generate_module_creator_function(&self, fn_name: &str, item: &IDLItem, module_name: &str) -> String {
        // 生成创建函数的具体实现
        format!(r#"fn {}(ctx: &Context) -> Result<JSValue> {{
    let obj = ctx.new_object()?;
    
    // 设置模块方法
    // TODO: 根据模块定义生成实际的方法设置代码
    
    Ok(obj.into())
}}"#, fn_name)
    }
}
```

#### 2.3 线程安全考虑

由于模块映射表需要在多线程环境中访问，使用了LazyLock和Mutex来确保线程安全：

- LazyLock确保映射表在首次访问时初始化
- Mutex保护对映射表的并发访问
- 在获取锁失败时返回错误，避免死锁

### 3. 代码生成流程

1. **解析RIDL**：识别module声明，将定义与模块信息关联
2. **生成模块注册表**：根据模块声明生成module_registry.rs文件
3. **生成Rust绑定**：为模块中的接口/类生成Rust实现
4. **生成C绑定**：生成对应的C函数用于JS调用
5. **注册require函数**：将require函数注册到global对象

### 4. singleton对象实现

对于singleton对象（如`singleton console`），不使用模块化机制，而是直接注册到global对象：

```
// 生成的Rust代码将singleton对象注册为global属性
context.add_global_object("console", console_object);
```

## 构建系统与构建流程

### 1. 多Cargo工程结构

为了支持复杂的系统，RIDL模块可以分散在不同目录中，每个模块作为一个独立的Cargo工程：

```
mquickjs-rs/
├── Cargo.toml (workspace定义)
├── deps/
│   ├── jidl-tool/ (RIDL工具)
│   ├── mquickjs-rs/ (Rust绑定)
│   └── mquickjs/ (C引擎)
├── features/
│   ├── network/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── network.ridl
│   ├── deviceinfo/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── deviceinfo.ridl
│   └── ui/
│       ├── Cargo.toml
│       ├── src/
│       └── ui.ridl
└── target/
```

### 2. Workspace配置

在主Cargo.toml中定义workspace：

```toml
[workspace]
members = [
    "deps/jidl-tool",
    "deps/mquickjs-rs",
    "features/network",
    "features/deviceinfo",
    "features/ui",
]
```

### 3. RIDL收集与处理流程

#### 3.1 RIDL文件扫描

jidl-tool需要扫描workspace中所有子工程的RIDL文件：

```rust
// jidl-tool/src/main.rs
use std::path::Path;
use std::fs;

fn find_all_ridl_files(workspace_root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ridl_files = Vec::new();
    
    // 扫描workspace中的所有成员
    for entry in fs::read_dir(workspace_root)? {
        let entry = entry?;
        let path = entry.path();
        
        // 检查是否为Cargo工程目录
        if path.is_dir() && path.join("Cargo.toml").exists() {
            // 在工程目录中搜索.ridl文件
            for ridl_entry in fs::read_dir(&path)? {
                let ridl_entry = ridl_entry?;
                let ridl_path = ridl_entry.path();
                
                if ridl_path.extension().map_or(false, |ext| ext == "ridl") {
                    ridl_files.push(ridl_path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    Ok(ridl_files)
}
```

#### 3.2 全局AST构建

收集所有RIDL文件后，构建全局AST上下文：

```rust
// jidl-tool/src/main.rs
fn build_global_context(ridl_files: &[String]) -> Result<GlobalContext, Error> {
    let mut global_context = GlobalContext::new();
    
    for file_path in ridl_files {
        let content = std::fs::read_to_string(file_path)?;
        let ast = parse_ridl(&content)?;
        global_context.merge(ast);
    }
    
    Ok(global_context)
}
```

### 4. 统一代码生成流程

#### 4.1 C代码生成（mqjs_stdlib.c）

对于有module声明的定义，生成对应的C代码结构：

```
// 生成的模块类ID
static mquickjs_ffi::JSClassID js_network_module_class_id;

// 生成的模块类定义
static mquickjs_ffi::JSClassDef js_network_module_class_def = {
    class_name: "NetworkModule\0".as_ptr() as *const i8,
    finalizer: Some(js_network_module_finalizer),
    gc_mark: None,
};

// 生成的模块方法定义
static mquickjs_ffi::JSCFunctionListEntry js_network_module_funcs[] = {
    JS_CFUNC_DEF("getStatus\0".as_ptr() as *const i8, 0, js_network_get_status),
    JS_CFUNC_DEF("connect\0".as_ptr() as *const i8, 1, js_network_connect),
};

// 生成的模块初始化函数
static int js_init_module_network(mquickjs_ffi::JSContext *ctx, mquickjs_ffi::JSModuleDef *m) {
    mquickjs_ffi::JS_NewClassID(&js_network_module_class_id);
    mquickjs_ffi::JS_NewClass(mquickjs_ffi::JS_GetRuntime(ctx), js_network_module_class_id, &js_network_module_class_def);

    mquickjs_ffi::JSValue proto = mquickjs_ffi::JS_NewObject(ctx);
    mquickjs_ffi::JS_SetPropertyFunctionList(ctx, proto, js_network_module_funcs, 
        sizeof(js_network_module_funcs) / sizeof(mquickjs_ffi::JSCFunctionListEntry));

    mquickjs_ffi::JSValue obj = mquickjs_ffi::JS_NewObjectProto(ctx, proto);
    // 注意：不提供构造函数，只能通过require获取
    if (m) {
        mquickjs_ffi::JS_SetModuleExport(ctx, m, "network\0".as_ptr() as *const i8, obj);
    }
    return 0;
}
```

#### 4.2 模块映射表生成

生成一个统一的require函数，以及模块名到ClassID的映射表：

```
// 模块映射表
typedef struct {
    const char* name;
    mquickjs_ffi::JSClassID* class_id_ptr;
} ModuleClassMapping;

static ModuleClassMapping module_mappings[] = {
    {"system.network\0".as_ptr() as *const i8, &js_network_module_class_id},
    {"system.deviceinfo\0".as_ptr() as *const i8, &js_deviceinfo_module_class_id},
    {NULL, NULL}  // 结束标记
};

// require函数实现
mquickjs_ffi::JSValue js_require(mquickjs_ffi::JSContext *ctx, mquickjs_ffi::JSValueConst this_val, 
                                 mquickjs_ffi::c_int argc, mquickjs_ffi::JSValueConst *argv) {
    const char *module_name = NULL;
    mquickjs_ffi::JS_ToCString(ctx, argv[0]);
    
    // 查找模块映射表
    for (int i = 0; module_mappings[i].name != NULL; i++) {
        if (strcmp(module_name, module_mappings[i].name) == 0) {
            // 创建模块对象实例
            mquickjs_ffi::JSValue obj = mquickjs_ffi::JS_NewObjectClass(ctx, *module_mappings[i].class_id_ptr);
            mquickjs_ffi::JS_FreeCString(ctx, module_name);
            return obj;
        }
    }
    
    mquickjs_ffi::JS_FreeCString(ctx, module_name);
    return mquickjs_ffi::JS_ThrowReferenceError(ctx, "Module '%s' not found\0".as_ptr() as *const i8, module_name);
}

// 在初始化时注册require函数
void js_register_require(mquickjs_ffi::JSContext *ctx, mquickjs_ffi::JSModuleDef *m) {
    mquickjs_ffi::JSValue global_obj = mquickjs_ffi::JS_GetGlobalObject(ctx);
    mquickjs_ffi::JS_SetPropertyStr(ctx, global_obj, "require\0".as_ptr() as *const i8, 
        mquickjs_ffi::JS_NewCFunction(ctx, js_require, "require\0".as_ptr() as *const i8, 1));
    mquickjs_ffi::JS_FreeValue(ctx, global_obj);
}
```

### 5. 构建流程

#### 5.1 构建步骤

完整的构建流程包括以下步骤：

1. **依赖准备**：构建mquickjs C库
   ```bash
   cd deps/mquickjs && make
   ```

2. **RIDL收集**：扫描workspace中所有.ridl文件
   ```bash
   # jidl-tool扫描所有子工程的RIDL文件
   jidl-tool --scan-workspace
   ```

3. **代码生成**：根据所有RIDL定义生成绑定代码
   ```bash
   jidl-tool --generate-bindings
   ```

4. **编译绑定**：编译生成的C/Rust代码到mqjs_stdlib静态库
   ```bash
   # 编译生成的绑定代码
   cargo build -p mquickjs-rs
   ```

5. **编译应用**：编译最终的应用程序
   ```bash
   cargo build
   ```

#### 5.2 构建脚本示例

创建构建脚本`build.sh`自动化整个流程：

```
#!/bin/bash
set -e

echo "Step 1: Building mquickjs C library..."
cd deps/mquickjs
make
cd ../..

echo "Step 2: Collecting RIDL files and generating bindings..."
cargo run -p jidl-tool -- --scan-workspace

echo "Step 3: Compiling generated bindings..."
cargo build -p mquickjs-rs

echo "Step 4: Compiling main application..."
cargo build

echo "Build completed successfully!"
```

#### 5.3 Cargo构建脚本（build.rs）

在mquickjs-rs的build.rs中集成绑定生成：

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    // 获取项目根目录
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_root = PathBuf::from(&project_root).parent().unwrap();
    
    // 扫描并生成绑定
    let ridl_files = find_all_ridl_files(&project_root);
    let global_context = build_global_context(&ridl_files).unwrap();
    
    // 生成C和Rust绑定代码
    generate_bindings(&global_context);
    
    // 链接到生成的静态库
    println!("cargo:rustc-link-search=native=deps/mquickjs");
    println!("cargo:rustc-link-lib=static=mquickjs");
    
    // 链接到生成的绑定库
    println!("cargo:rustc-link-lib=static=mqjs_stdlib");
}
```


## 相关文档

- [RIDL_DESIGN.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/RIDL_DESIGN.md) - RIDL设计文档，提供设计原则和语法设计背景
- [RIDL_GRAMMAR_SPEC.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/RIDL_GRAMMAR_SPEC.md) - 词法和文法规范，提供详细语法定义
- [FEATURE_DEVELOPMENT_GUIDE.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/FEATURE_DEVELOPMENT_GUIDE.md) - 如何开发和集成基于RIDL的Feature模块
- [TECH_SELECTION.md](file:///home/peng/workspace/mquickjs-rs-demo/deps/jidl-tool/doc/TECH_SELECTION.md) - jidl-tool的技术选型和实现计划