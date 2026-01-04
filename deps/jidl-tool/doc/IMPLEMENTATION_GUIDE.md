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
```idl
json struct Person {
    string name;
    int age;
    string? email;
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
```idl
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
```c
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
```idl
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
```c
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
```idl
typedef ProcessCallback = callback(string | object result, bool success);

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
```c
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
```rust
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
```rust
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
- **可选类型**：使用 `Option<T>`，映射到 JS 的 `null/undefined`
- **字典类型**：使用 `HashMap<String, V>`，映射到 JS 对象

### 3. 转换辅助函数

```rust
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