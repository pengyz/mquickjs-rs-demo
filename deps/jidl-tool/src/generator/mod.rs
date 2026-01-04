use crate::parser::ast::{IDLItem, Interface, Class, Enum, Function, Type, Field, Property, Method, StructDef, SerializationFormat, PropertyModifier};
use std::fs;
use std::path::Path;

/// 生成代码
pub fn generate_code(items: &[IDLItem], output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new(output_dir);
    
    // 创建输出目录
    fs::create_dir_all(output_path)?;
    
    // 生成Rust胶水代码
    generate_rust_glue(items, output_path)?;
    
    // 生成C绑定代码
    generate_c_bindings(items, output_path)?;
    
    // 生成标准库描述代码
    generate_stdlib_descriptions(items, output_path)?;
    
    Ok(())
}

fn generate_rust_glue(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let rust_glue_path = output_path.join("rust_glue.rs");
    let mut content = String::new();
    
    content.push_str("// Auto-generated Rust glue code\n\n");
    
    for item in items {
        match item {
            IDLItem::Interface(interface) => {
                content.push_str(&generate_interface_glue(interface));
            }
            IDLItem::Class(class) => {
                content.push_str(&generate_class_glue(class));
            }
            IDLItem::Enum(enum_def) => {
                content.push_str(&generate_enum_glue(enum_def));
            }
            IDLItem::Struct(struct_def) => {
                content.push_str(&generate_struct_glue(struct_def));
            }
            IDLItem::GlobalFunction(function) => {
                content.push_str(&generate_function_glue(function));
            }
        }
    }
    
    fs::write(rust_glue_path, content)?;
    Ok(())
}

fn generate_c_bindings(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let c_bindings_path = output_path.join("c_bindings.c");
    let mut content = String::new();
    
    content.push_str("// Auto-generated C bindings\n\n");
    content.push_str("#include \"mquickjs.h\"\n\n");
    
    for item in items {
        match item {
            IDLItem::Interface(interface) => {
                content.push_str(&generate_interface_c_bindings(interface));
            }
            IDLItem::Class(class) => {
                content.push_str(&generate_class_c_bindings(class));
            }
            IDLItem::Enum(enum_def) => {
                content.push_str(&generate_enum_c_bindings(enum_def));
            }
            IDLItem::Struct(struct_def) => {
                content.push_str(&generate_struct_c_bindings(struct_def));
            }
            IDLItem::GlobalFunction(function) => {
                content.push_str(&generate_function_c_bindings(function));
            }
        }
    }
    
    fs::write(c_bindings_path, content)?;
    Ok(())
}

fn generate_stdlib_descriptions(items: &[IDLItem], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stdlib_desc_path = output_path.join("stdlib_desc.c");
    let mut content = String::new();
    
    content.push_str("/* Auto-generated standard library description */\n");
    content.push_str("#include \"mquickjs/mquickjs.h\"\n\n");
    
    // 声明所有类
    for item in items {
        match item {
            IDLItem::Class(class) => {
                content.push_str(&format!("extern const JSClassDef {}_class;\n", class.name.to_lowercase()));
            }
            IDLItem::GlobalFunction(function) => {
                content.push_str(&format!("JSValue {}(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n", function.name));
            }
            _ => {}
        }
    }
    content.push_str("\n");
    
    // 生成类初始化函数
    for item in items {
        if let IDLItem::Class(class) = item {
            content.push_str(&format!(
                "static JSValue js_{}_constructor(JSContext *ctx, JSValue new_target, int argc, JSValue *argv) {{\n    // 委托到 Rust 实现\n    return {}_constructor(ctx, new_target, argc, argv);\n}}\n\n",
                class.name.to_lowercase(),
                class.name.to_lowercase()
            ));
        }
    }
    
    // 生成全局对象定义
    content.push_str("static const JSCPropDef js_global_object[] = {\n");
    
    // 自定义类
    for item in items {
        if let IDLItem::Class(class) = item {
            content.push_str(&format!("    JS_PROP_CLASS_DEF(\"{}\", &{}_class),\n", 
                class.name, class.name.to_lowercase()));
        }
    }
    
    // 全局函数
    for item in items {
        if let IDLItem::GlobalFunction(function) = item {
            content.push_str(&format!("    JS_CFUNC_DEF(\"{}\", 0, {}),\n", 
                function.name, 
                function.name));
        }
    }
    
    content.push_str("    JS_PROP_END,\n};\n\n");
    
    // 生成初始化函数
    content.push_str("int js_stdlib_init(JSContext *ctx, JSValue global_obj) {\n");
    for item in items {
        if let IDLItem::Class(class) = item {
            content.push_str(&format!("    if (JS_NewClass(ctx, js_{}_class.class_id, &{}_class) < 0)\n        return -1;\n", 
                class.name.to_lowercase(), class.name.to_lowercase()));
        }
    }
    
    content.push_str("    if (JS_SetPropertyFunctionList(ctx, global_obj, js_global_object,\n                  sizeof(js_global_object) / sizeof(JSCPropDef)) < 0)\n        return -1;\n\n");
    content.push_str("    return 0;\n}\n");
    
    fs::write(stdlib_desc_path, content)?;
    Ok(())
}

// 辅助函数：生成各种类型的胶水代码
fn generate_interface_glue(interface: &Interface) -> String {
    let mut code = format!("// Interface: {}\n", interface.name);
    code.push_str(&format!("pub struct {} {{\n", interface.name));
    code.push_str("    // Interface implementation\n");
    code.push_str("}\n\n");
    
    for method in &interface.methods {
        code.push_str(&generate_method_glue(&interface.name, method));
    }
    
    code
}

fn generate_class_glue(class: &Class) -> String {
    let mut content = String::new();
    
    content.push_str(&format!("// Class {}\n", class.name));
    
    // 生成结构体定义
    content.push_str(&format!("pub struct {} {{\n", class.name));
    for prop in &class.properties {
        let field_type = match &prop.property_type {
            Type::Int => "i32",
            Type::Float => "f64",
            Type::String => "String",
            Type::Bool => "bool",
            Type::Array(boxed_type) => match boxed_type.as_ref() {
                Type::String => "Vec<String>",
                Type::Int => "Vec<i32>",
                _ => "Vec<String>", // 默认
            },
            _ => "String", // 默认类型
        };
        
        // 检查是否是普通属性（没有特殊修饰符）
        let is_normal_property = !prop.modifiers.contains(&PropertyModifier::Const) &&
                                !prop.modifiers.contains(&PropertyModifier::Readonly) &&
                                !prop.modifiers.contains(&PropertyModifier::ReadWrite);
        
        if is_normal_property {
            content.push_str(&format!("    pub {}: {},\n", prop.name, field_type));
        } else {
            // 对于property、readonly property和const，我们只在结构体中存储实际数据
            if prop.modifiers.contains(&PropertyModifier::ReadWrite) || 
               prop.modifiers.contains(&PropertyModifier::Readonly) ||
               prop.modifiers.contains(&PropertyModifier::Const) {
                content.push_str(&format!("    pub {}: {},\n", prop.name, field_type));
            }
        }
    }
    content.push_str("}\n\n");
    
    // 生成实现块
    content.push_str(&format!("impl {} {{\n", class.name));
    
    // 生成构造函数
    if let Some(constructor) = &class.constructor {
        content.push_str(&format!("    pub fn new({}) -> Self {{\n", 
            constructor.params.iter()
                .map(|p| format!("{}: {}", p.name, rust_type_from_idl(&p.param_type)))
                .collect::<Vec<_>>()
                .join(", ")));
        
        content.push_str(&format!("        {} {{\n", class.name));
        for prop in &class.properties {
            if prop.modifiers.contains(&PropertyModifier::ReadWrite) || 
               prop.modifiers.contains(&PropertyModifier::Readonly) ||
               prop.modifiers.contains(&PropertyModifier::Const) {
                // 对于可读写、只读和常量属性，需要初始化
                let default_value = match &prop.property_type {
                    Type::Int => "0",
                    Type::Float => "0.0",
                    Type::String => "\"\".to_string()",
                    Type::Bool => "false",
                    Type::Array(_) => "Vec::new()",
                    _ => "\"\".to_string()", // 默认
                };
                content.push_str(&format!("            {}: {},\n", prop.name, default_value));
            } else {
                // 普通属性的默认值
                let default_value = match &prop.property_type {
                    Type::Int => "0",
                    Type::Float => "0.0",
                    Type::String => "\"\".to_string()",
                    Type::Bool => "false",
                    Type::Array(_) => "Vec::new()",
                    _ => "\"\".to_string()", // 默认
                };
                content.push_str(&format!("            {}: {},\n", prop.name, default_value));
            }
        }
        content.push_str("        }\n");
        content.push_str("    }\n\n");
    }
    
    // 生成getter和setter方法
    for prop in &class.properties {
        match prop.modifiers.first() {
            Some(PropertyModifier::ReadWrite) => {
                // 生成getter
                content.push_str(&format!("    pub fn get_{}(&self) -> {} {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                content.push_str(&format!("        self.{}.clone()\n", prop.name));
                content.push_str("    }\n\n");
                
                // 生成setter
                content.push_str(&format!("    pub fn set_{}(&mut self, value: {}) {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                content.push_str(&format!("        self.{} = value;\n", prop.name));
                content.push_str("    }\n\n");
            }
            Some(PropertyModifier::Readonly) => {
                // 仅生成getter
                content.push_str(&format!("    pub fn get_{}(&self) -> {} {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                content.push_str(&format!("        self.{}.clone()\n", prop.name));
                content.push_str("    }\n\n");
            }
            Some(PropertyModifier::Const) => {
                // 生成getter返回常量值
                content.push_str(&format!("    pub fn get_{}(&self) -> {} {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                if let Some(ref default_value) = prop.default_value {
                    content.push_str(&format!("        {}\n", default_value)); // 返回常量值
                } else {
                    content.push_str("        todo!() // const value not specified\n");
                }
                content.push_str("    }\n\n");
            }
            _ => {
                // 普通属性，生成getter和setter
                content.push_str(&format!("    pub fn get_{}(&self) -> {} {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                content.push_str(&format!("        self.{}.clone()\n", prop.name));
                content.push_str("    }\n\n");
                
                content.push_str(&format!("    pub fn set_{}(&mut self, value: {}) {{\n", 
                    prop.name, rust_type_from_idl(&prop.property_type)));
                content.push_str(&format!("        self.{} = value;\n", prop.name));
                content.push_str("    }\n\n");
            }
        }
    }
    
    // 生成用户定义的方法
    for method in &class.methods {
        content.push_str(&format!("    pub fn {}(&self) {{\n", method.name));
        content.push_str("        todo!()\n");
        content.push_str("    }\n\n");
    }
    
    content.push_str("}\n\n");
    
    // 生成C绑定的外部函数声明
    content.push_str(&generate_c_glue_functions(class));
    
    content
}

fn generate_enum_glue(enum_def: &Enum) -> String {
    let mut code = format!("// Enum: {}\n", enum_def.name);
    code.push_str(&format!("pub enum {} {{\n", enum_def.name));
    
    for value in &enum_def.values {
        if let Some(val) = value.value {
            code.push_str(&format!("    {} = {},\n", value.name, val));
        } else {
            code.push_str(&format!("    {},\n", value.name));
        }
    }
    
    code.push_str("}\n\n");
    code
}

fn generate_struct_glue(struct_def: &StructDef) -> String {
    let mut code = format!("// Struct: {}\n", struct_def.name);
    
    // 添加序列化相关导入
    match struct_def.serialization_format {
        SerializationFormat::Json => {
            code.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
        }
        SerializationFormat::MsgPack => {
            code.push_str("#[derive(serde::Serialize, serde::Deserialize)]\n");
        }
        SerializationFormat::Protobuf => {
            code.push_str("#[derive(protobuf::Message)]\n");
        }
    }
    
    code.push_str(&format!("pub struct {} {{\n", struct_def.name));
    
    for field in &struct_def.fields {
        code.push_str(&format!("    pub {}: {},\n", field.name, rust_type_name(&field.field_type)));
    }
    
    code.push_str("}\n\n");
    code
}

fn generate_function_glue(function: &Function) -> String {
    let params = function
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, rust_type_name(&p.param_type)))
        .collect::<Vec<_>>()
        .join(", ");
    
    format!(
        "pub fn {}({}) -> {} {{\n    todo!(\"Function implementation\");\n}}\n\n",
        function.name,
        params,
        rust_type_name(&function.return_type)
    )
}

fn generate_method_glue(class_name: &str, method: &Method) -> String {
    let params = method
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, rust_type_name(&p.param_type)))
        .collect::<Vec<_>>()
        .join(", ");
    
    format!(
        "impl {} {{\n    pub fn {}(&self, {}) -> {} {{\n        todo!(\"Method implementation\");\n    }}\n}}\n\n",
        class_name,
        method.name,
        params,
        rust_type_name(&method.return_type)
    )
}

fn generate_constructor_glue(class_name: &str, constructor: &Function) -> String {
    let params = constructor
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, rust_type_name(&p.param_type)))
        .collect::<Vec<_>>()
        .join(", ");
    
    format!(
        "impl {} {{\n    pub fn new({}) -> Self {{\n        todo!(\"Constructor implementation\");\n    }}\n}}\n\n",
        class_name,
        params
    )
}

// 辅助函数：生成C绑定代码
fn generate_interface_c_bindings(interface: &Interface) -> String {
    let mut code = format!("// C bindings for interface: {}\n", interface.name);
    
    for method in &interface.methods {
        code.push_str(&format!(
            "// Method: {}::{}\nJSValue {}_{}_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            interface.name, method.name, interface.name, method.name
        ));
    }
    
    code
}

fn generate_class_c_bindings(class: &Class) -> String {
    let mut code = format!("// C bindings for class: {}\n", class.name);
    
    if let Some(ref constructor) = class.constructor {
        code.push_str(&format!(
            "// Constructor: {}\nJSValue {}_constructor(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            class.name, class.name
        ));
    }
    
    for method in &class.methods {
        code.push_str(&format!(
            "// Method: {}::{}\nJSValue {}_{}_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
            class.name, method.name, class.name, method.name
        ));
    }
    
    code
}

fn generate_enum_c_bindings(enum_def: &Enum) -> String {
    format!("// C bindings for enum: {}\n\n", enum_def.name)
}

fn generate_struct_c_bindings(struct_def: &StructDef) -> String {
    format!("// C bindings for struct: {}\n\n", struct_def.name)
}

fn generate_function_c_bindings(function: &Function) -> String {
    format!(
        "// Global function: {}\nJSValue js_{}(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);\n\n",
        function.name, function.name
    )
}

fn generate_c_class_definition(class: &Class) -> String {
    let mut content = String::new();
    
    // 生成原型属性定义
    content.push_str(&format!("/* {} class prototype properties */\n", class.name));
    content.push_str(&format!("static const JSCPropDef {}_proto_props[] = {{\n", class.name.to_lowercase()));
    
    for prop in &class.properties {
        match prop.modifiers.first() {
            Some(PropertyModifier::ReadWrite) => {
                content.push_str(&format!("    JS_CGETSET_DEF(\"{}\", {}_get_{}, {}_set_{}),\n", 
                    prop.name, class.name.to_lowercase(), prop.name, class.name.to_lowercase(), prop.name));
            }
            Some(PropertyModifier::Readonly) => {
                content.push_str(&format!("    JS_CGETSET_DEF(\"{}\", {}_get_{}, NULL),\n", 
                    prop.name, class.name.to_lowercase(), prop.name));
            }
            _ => {} // 普通属性和const属性不需要在这里定义
        }
    }
    
    // 生成方法
    for method in &class.methods {
        content.push_str(&format!("    JS_CFUNC_DEF(\"{}\", 0, {}_{}),\n", 
            method.name, class.name.to_lowercase(), method.name));
    }
    
    content.push_str("    JS_PROP_END,\n};\n\n");
    
    // 生成类定义
    content.push_str(&format!(
        "static const JSClassDef {}_class = {{\n    \"{}\",\n    .class_init = {}_constructor,\n    .finalizer = {}_finalizer,\n    .gc_mark = NULL,\n    .call = NULL,\n    .exotic = NULL,\n}};\n\n",
        class.name.to_lowercase(),
        class.name,
        class.name.to_lowercase(),
        class.name.to_lowercase()
    ));
    
    content
}

fn generate_c_glue_functions(class: &Class) -> String {
    let mut content = String::new();
    
    // 生成构造函数的C绑定
    if let Some(constructor) = &class.constructor {
        content.push_str(&format!(
            "extern \"C\" fn {}_constructor(\n    ctx: *mut mquickjs_ffi::JSContext,\n    _new_target: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n", 
            class.name.to_lowercase()
        ));
        
        // 从JS获取参数
        for (i, param) in constructor.params.iter().enumerate() {
            content.push_str(&format!(
                "    let {}_js = if argc > {} {{ unsafe {{ *argv.offset({} as isize) }} }} else {{ unsafe {{ mquickjs_ffi::JS_UNDEFINED }} }};\n",
                param.name, i, i
            ));
            content.push_str(&format!(
                "    let {} = get_{}_from_js_value(ctx, {}_js).unwrap_or_default();\n",
                param.name, js_type_from_idl(&param.param_type), param.name
            ));
        }
        
        // 创建Rust对象实例
        let params_for_constructor = constructor.params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
            
        content.push_str(&format!("    let {}_instance = {}::new({});\n", 
            class.name.to_lowercase(), class.name, params_for_constructor));
        
        // 使用 Box 将实例分配在堆上
        content.push_str(&format!("    let boxed_instance = Box::new({}_instance);\n", class.name.to_lowercase()));
        content.push_str("    let instance_ptr = Box::into_raw(boxed_instance);\n\n");
        
        // 将指针存储到JS对象中
        content.push_str("    unsafe {\n");
        content.push_str("        mquickjs_ffi::JS_SetOpaque(ctx, _new_target, instance_ptr as *mut std::ffi::c_void);\n");
        content.push_str("    }\n\n");
        
        // 为const属性设置常量值
        for prop in &class.properties {
            if prop.modifiers.contains(&PropertyModifier::Const) {
                if let Some(ref default_value) = prop.default_value {
                    content.push_str(&format!(
                        "    unsafe {{ mquickjs_ffi::JS_SetPropertyStr(ctx, _new_target, \"{}\", mquickjs_ffi::JS_New{}(ctx, {})); }}\n",
                        prop.name,
                        js_type_name_from_idl(&prop.property_type),
                        default_value
                    ));
                }
            }
        }
        
        content.push_str("    _new_target\n");
        content.push_str("}\n\n");
    }
    
    // 为每个property生成getter和setter
    for prop in &class.properties {
        let is_normal_property = !prop.modifiers.contains(&PropertyModifier::Const) &&
                                !prop.modifiers.contains(&PropertyModifier::Readonly) &&
                                !prop.modifiers.contains(&PropertyModifier::ReadWrite);
        
        match prop.modifiers.first() {
            Some(PropertyModifier::ReadWrite) | Some(_) if is_normal_property => {
                // 生成getter
                content.push_str(&format!(
                    "extern \"C\" fn {}_get_{}(\n    ctx: *mut mquickjs_ffi::JSContext,\n    this_val: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n",
                    class.name.to_lowercase(),
                    prop.name
                ));
                content.push_str(&format!(
                    "    let instance_ptr = unsafe {{ mquickjs_ffi::JS_GetOpaque(ctx, this_val) as *mut {} }};\n",
                    class.name
                ));
                content.push_str("    let instance = unsafe { &*instance_ptr };\n");
                content.push_str(&format!("    let value = instance.get_{}();\n", prop.name));
                content.push_str(&format!(
                    "    let ctx_ref = unsafe {{ &*ctx }};\n    match ctx_ref.create_{}(value) {{\n        Ok(js_value) => js_value.value,\n        Err(_) => unsafe {{ mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, \"Failed to create {}\")) }}\n    }}\n",
                    js_type_from_idl(&prop.property_type),
                    js_type_from_idl(&prop.property_type)
                ));
                content.push_str("}\n\n");
                
                // 生成setter
                content.push_str(&format!(
                    "extern \"C\" fn {}_set_{}(\n    ctx: *mut mquickjs_ffi::JSContext,\n    this_val: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n",
                    class.name.to_lowercase(),
                    prop.name
                ));
                content.push_str(&format!(
                    "    let instance_ptr = unsafe {{ mquickjs_ffi::JS_GetOpaque(ctx, this_val) as *mut {} }};\n",
                    class.name
                ));
                content.push_str("    let instance = unsafe { &mut *instance_ptr };\n");
                content.push_str("    let value_js = if argc > 0 { unsafe { *argv.offset(0) } } else { unsafe { mquickjs_ffi::JS_UNDEFINED } };\n");
                content.push_str(&format!(
                    "    let value = get_{}_from_js_value(ctx, value_js).unwrap_or_default();\n",
                    js_type_from_idl(&prop.property_type)
                ));
                content.push_str(&format!("    instance.set_{}(value);\n", prop.name));
                content.push_str("    unsafe { mquickjs_ffi::JS_UNDEFINED }\n");
                content.push_str("}\n\n");
            }
            Some(PropertyModifier::Readonly) => {
                // 仅生成getter
                content.push_str(&format!(
                    "extern \"C\" fn {}_get_{}(\n    ctx: *mut mquickjs_ffi::JSContext,\n    this_val: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n",
                    class.name.to_lowercase(),
                    prop.name
                ));
                content.push_str(&format!(
                    "    let instance_ptr = unsafe {{ mquickjs_ffi::JS_GetOpaque(ctx, this_val) as *mut {} }};\n",
                    class.name
                ));
                content.push_str("    let instance = unsafe { &*instance_ptr };\n");
                content.push_str(&format!("    let value = instance.get_{}();\n", prop.name));
                content.push_str(&format!(
                    "    let ctx_ref = unsafe {{ &*ctx }};\n    match ctx_ref.create_{}(value) {{\n        Ok(js_value) => js_value.value,\n        Err(_) => unsafe {{ mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, \"Failed to create {}\")) }}\n    }}\n",
                    js_type_from_idl(&prop.property_type),
                    js_type_from_idl(&prop.property_type)
                ));
                content.push_str("}\n\n");
            }
            Some(PropertyModifier::Const) => {
                // const属性不需要C绑定的getter/setter，因为它们在构造时就设为JS对象的属性
            }
            _ => {
                // 普通属性，生成getter和setter
                content.push_str(&format!(
                    "extern \"C\" fn {}_get_{}(\n    ctx: *mut mquickjs_ffi::JSContext,\n    this_val: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n",
                    class.name.to_lowercase(),
                    prop.name
                ));
                content.push_str(&format!(
                    "    let instance_ptr = unsafe {{ mquickjs_ffi::JS_GetOpaque(ctx, this_val) as *mut {} }};\n",
                    class.name
                ));
                content.push_str("    let instance = unsafe { &*instance_ptr };\n");
                content.push_str(&format!("    let value = instance.get_{}();\n", prop.name));
                content.push_str(&format!(
                    "    let ctx_ref = unsafe {{ &*ctx }};\n    match ctx_ref.create_{}(value) {{\n        Ok(js_value) => js_value.value,\n        Err(_) => unsafe {{ mquickjs_ffi::JS_Throw(ctx, create_js_error(ctx, \"Failed to create {}\")) }}\n    }}\n",
                    js_type_from_idl(&prop.property_type),
                    js_type_from_idl(&prop.property_type)
                ));
                content.push_str("}\n\n");
                
                content.push_str(&format!(
                    "extern \"C\" fn {}_set_{}(\n    ctx: *mut mquickjs_ffi::JSContext,\n    this_val: mquickjs_ffi::JSValue,\n    argc: mquickjs_ffi::c_int,\n    argv: *mut mquickjs_ffi::JSValue\n) -> mquickjs_ffi::JSValue {{\n",
                    class.name.to_lowercase(),
                    prop.name
                ));
                content.push_str(&format!(
                    "    let instance_ptr = unsafe {{ mquickjs_ffi::JS_GetOpaque(ctx, this_val) as *mut {} }};\n",
                    class.name
                ));
                content.push_str("    let instance = unsafe { &mut *instance_ptr };\n");
                content.push_str("    let value_js = if argc > 0 { unsafe { *argv.offset(0) } } else { unsafe { mquickjs_ffi::JS_UNDEFINED } };\n");
                content.push_str(&format!(
                    "    let value = get_{}_from_js_value(ctx, value_js).unwrap_or_default();\n",
                    js_type_from_idl(&prop.property_type)
                ));
                content.push_str(&format!("    instance.set_{}(value);\n", prop.name));
                content.push_str("    unsafe { mquickjs_ffi::JS_UNDEFINED }\n");
                content.push_str("}\n\n");
            }
        }
    }
    
    // 生成类的finalizer
    content.push_str(&format!(
        "extern \"C\" fn {}_finalizer(\n    ctx: *mut mquickjs_ffi::JSContext,\n    opaque: *mut std::ffi::c_void\n) {{\n",
        class.name.to_lowercase()
    ));
    content.push_str(&format!(
        "    let instance = unsafe {{ Box::from_raw(opaque as *mut {}) }};\n",
        class.name
    ));
    content.push_str("    drop(instance);\n");
    content.push_str("}\n\n");
    
    content
}

// 辅助函数：将IDL类型转换为Rust类型名
fn rust_type_name(idl_type: &Type) -> String {
    match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),
        Type::Any => "serde_json::Value".to_string(),
        Type::Array(t) => format!("Vec<{}>", rust_type_name(t)),
        Type::Map(k, v) => format!("std::collections::HashMap<{}, {}>", rust_type_name(k), rust_type_name(v)),
        Type::Union(_) => "serde_json::Value".to_string(), // Union types use JSON for flexibility
        Type::Optional(t) => format!("Option<{}>", rust_type_name(t)),
        Type::Custom(name) => name.clone(),
        Type::Callback(_) => "Box<dyn Fn()>".to_string(), // Simplified callback representation
    }
}

fn rust_type_from_idl(idl_type: &Type) -> String {
    match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),
        Type::Any => "serde_json::Value".to_string(),
        Type::Array(inner) => format!("Vec<{}>", rust_type_from_idl(inner)),
        Type::Map(key, value) => format!("std::collections::HashMap<{}, {}>", 
            rust_type_from_idl(key), rust_type_from_idl(value)),
        Type::Union(_) => "serde_json::Value".to_string(), // Union types use JSON value
        Type::Optional(inner) => format!("Option<{}>", rust_type_from_idl(inner)),
        Type::Custom(name) => name.clone(),
        Type::Callback(_) => "Box<dyn Fn()>".to_string(), // Simplified callback representation
    }
}

fn js_type_from_idl(idl_type: &Type) -> String {
    match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "int32".to_string(),
        Type::Float => "float64".to_string(),
        Type::String => "string".to_string(),
        _ => "any".to_string(), // Default to any for complex types
    }
}

fn js_type_name_from_idl(idl_type: &Type) -> String {
    match idl_type {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int32".to_string(),
        Type::Float => "Float64".to_string(),
        Type::String => "String".to_string(),
        _ => "Any".to_string(), // Default to any for complex types
    }
}
