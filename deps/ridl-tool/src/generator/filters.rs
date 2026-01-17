use crate::generator::code_writer::CodeWriter;
use crate::generator::TemplateParam;
use crate::parser::ast::{PropertyModifier, Type};
use crate::parser::FileMode;

// Generator template filters.
//
// IMPORTANT:
// - These functions may be used only by Askama templates (not necessarily referenced by Rust code).
// - Keep the set minimal and delete unused filters to avoid accumulating stale APIs.

pub fn length<T>(slice: &[T]) -> ::askama::Result<usize> {
    Ok(slice.len())
}


pub fn rust_type_from_idl(idl_type: &Type) -> Result<String, askama::Error> {
    let rust_type = match idl_type {
        Type::Bool => "bool".to_string(),
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Double => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),

        // Optional(T) at Rust boundary.
        Type::Optional(inner) => format!("Option<{}>", rust_type_from_idl(inner)?),

        // `any` at Rust boundary:
        // - param: borrowed view (Local<Value>)
        // - return: owned/rooted value (Global<Value>)
        //
        // Note: return-type mapping is handled at template level because it depends on whether
        // the position is param vs return.
        Type::Any => "mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>".to_string(),

        Type::Custom(name) => name.clone(),

        // Keep explicit: fail fast for types we haven't implemented yet.
        other => {
            return Err(askama::Error::Custom(
                format!("unsupported ridl type in rust_type_from_idl: {other:?}").into(),
            ));
        }
    };
    Ok(rust_type)
}

pub fn emit_value_to_js(ty: &Type, value_expr: &str) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::Bool => {
            w.push_line(format!(
                "mquickjs_rs::mquickjs_ffi::js_mkbool(({value}) != 0)",
                value = value_expr
            ));
        }
        Type::Int => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, {value}) }}",
                value = value_expr
            ));
        }
        Type::Double | Type::Float => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, {value}) }}",
                value = value_expr
            ));
        }
        Type::String => {
            w.push_line(format!(
                "let cstr = CString::new({value}).unwrap_or_else(|_| CString::new(\"\").unwrap());",
                value = value_expr
            ));
            w.push_line("unsafe { mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) }");
        }
        _ => {
            w.push_line("compile_error!(\"v1 glue: unsupported value conversion\");");
            w.push_line("mquickjs_rs::mquickjs_ffi::JS_UNDEFINED");
        }
    }

    Ok(w.into_string())
}

pub fn emit_return_convert(return_type: &Type, result_name: &str) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match return_type {
        Type::Void => {
            w.push_line(format!("let _ = {result_name};", result_name = result_name));
            w.push_line("let _ = ctx;");
            w.push_line("let _ = argc;");
            w.push_line("let _ = argv;");
            w.push_line("mquickjs_rs::mquickjs_ffi::JS_UNDEFINED");
        }
        Type::String => {
            w.push_line(format!(
                "let cstr = CString::new({result_name}).unwrap_or_else(|_| CString::new(\"\").unwrap());",
                result_name = result_name
            ));
            w.push_line("unsafe { mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) }");
        }
        Type::Int => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, {result_name}) }}",
                result_name = result_name
            ));
        }
        Type::Bool => {
            w.push_line(format!(
                "mquickjs_rs::mquickjs_ffi::js_mkbool({result_name})",
                result_name = result_name
            ));
        }
        Type::Double | Type::Float => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, {result_name}) }}",
                result_name = result_name
            ));
        }
        Type::Any => {
            w.push_line(format!("{result_name}.as_raw()", result_name = result_name));
        }
        _ => {
            w.push_line("compile_error!(\"v1 glue: unsupported return type\");");
            w.push_line("mquickjs_rs::mquickjs_ffi::JS_UNDEFINED");
        }
    }

    Ok(w.into_string())
}

pub fn is_readonly_prop(modifiers: &[PropertyModifier]) -> ::askama::Result<bool> {
    Ok(modifiers.contains(&PropertyModifier::ReadOnly))
}

pub fn is_proto_prop(modifiers: &[PropertyModifier]) -> ::askama::Result<bool> {
    Ok(modifiers.contains(&PropertyModifier::Proto))
}

pub fn any_proto_props(properties: &[crate::parser::ast::Property]) -> ::askama::Result<bool> {
    Ok(properties
        .iter()
        .any(|p| p.modifiers.contains(&PropertyModifier::Proto)))
}

pub fn normalize_ident(s: &str) -> ::askama::Result<String> {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    Ok(out)
}

pub fn to_snake_case(s: &str) -> ::askama::Result<String> {
    Ok(crate::generator::naming::to_snake_case(s))
}

pub fn to_upper_camel_case(s: &str) -> ::askama::Result<String> {
    Ok(crate::generator::naming::to_upper_camel_case(s))
}

#[allow(dead_code)]
pub fn proto_module_ns(
    module_decl: &Option<crate::parser::ast::ModuleDeclaration>,
    fallback_module_name: &str,
) -> ::askama::Result<String> {
    Ok(module_decl
        .as_ref()
        .map(|m| m.module_path.clone())
        .unwrap_or_else(|| fallback_module_name.to_string()))
}

pub fn methods_total(
    interfaces: &[crate::generator::TemplateInterface],
    classes: &[crate::generator::TemplateClass],
) -> ::askama::Result<usize> {
    let mut total = 0usize;
    for i in interfaces {
        total += i.methods.len();
    }
    for c in classes {
        total += c.methods.len();
    }
    Ok(total)
}

pub fn methods_total_filter(
    interfaces: &[crate::generator::TemplateInterface],
    classes: &[crate::generator::TemplateClass],
) -> ::askama::Result<usize> {
    methods_total(interfaces, classes)
}

pub fn emit_setter_value_extract(prop: &crate::parser::ast::Property) -> ::askama::Result<String> {
    // Contract: setter takes exactly one argument at argv[0].
    // We intentionally do not use `this_val` here.
    let mut w = CodeWriter::new();

    emit_missing_arg(&mut w, 1, "value");

    // argv[0] -> v0
    w.push_line("let v0 = unsafe { *argv.add(0) };".to_string());

    // Convert v0 into `v0` (Rust typed) in-place.
    match &prop.property_type {
        Type::Bool => {
            w.push_line("let v0: bool = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToBool(ctx, v0) } != 0;".to_string());
        }
        Type::Int => {
            emit_check_is_number_expr(&mut w, "v0", "\"arg1: expected number\"");
            // Avoid shadowing the JSValue `v0`.
            emit_to_i32_expr(&mut w, "v0", "out0", "\"arg1: failed to convert to int\"");
            w.push_line("let v0: i32 = out0;".to_string());
        }
        Type::Double | Type::Float => {
            emit_check_is_number_expr(&mut w, "v0", "\"arg1: expected number\"");
            emit_to_f64_expr(&mut w, "v0", "v0", "\"arg1: failed to convert to number\"");
            if matches!(prop.property_type, Type::Float) {
                w.push_line("let v0: f32 = v0 as f32;".to_string());
            }
        }
        Type::String => {
            emit_check_is_string_expr(&mut w, "v0", "\"arg1: expected string\"");
            // Convert to C string pointer; in this mquickjs fork, JS_ToCString returns a borrowed pointer.
            w.push_line("let mut buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf { buf: [0u8; 5] };".to_string());
            w.push_line("let cptr = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v0, &mut buf as *mut _) };".to_string());
            w.push_line("if cptr.is_null() { return js_throw_type_error(ctx, \"arg1: failed to convert to string\"); }".to_string());
            w.push_line("let v0: *const core::ffi::c_char = cptr;".to_string());
        }
        _ => {
            w.push_line("return js_throw_type_error(ctx, \"setter: unsupported property type\");".to_string());
        }
    }

    Ok(w.into_string())
}


pub fn emit_param_extract(
    param: &TemplateParam,
    idx0: &usize,
    idx1: &usize,
) -> ::askama::Result<String> {
    if param.variadic {
        return emit_varargs_collect(&param.name, &param.ty, param.file_mode, *idx0);
    }

    // v1 glue: minimal support for `any?` (Optional(Any)) used by tests.
    if matches!(&param.ty, Type::Optional(inner) if matches!(inner.as_ref(), Type::Any)) {
        let mut w = CodeWriter::new();
        emit_missing_arg(&mut w, *idx1, &param.name);
        emit_argv_v_let(&mut w, *idx0);

        match param.file_mode {
            FileMode::Default => {
                w.push_line(format!(
                    "let {name}: Option<JSValue> = if mquickjs_rs::mquickjs_ffi::JS_IsNull(_v) == 1 || mquickjs_rs::mquickjs_ffi::JS_IsUndefined(_v) == 1 {{ None }} else {{ Some(_v) }};",
                    name = param.name
                ));
            }
            FileMode::Strict => {
                w.push_line(format!(
                    "let {name}: Option<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>> = if mquickjs_rs::mquickjs_ffi::JS_IsNull(_v) == 1 || mquickjs_rs::mquickjs_ffi::JS_IsUndefined(_v) == 1 {{ None }} else {{ Some(scope.value(_v)) }};",
                    name = param.name
                ));
            }
        }

        return Ok(w.into_string());
    }

    emit_single_param_extract(&param.name, &param.ty, param.file_mode, *idx0, *idx1)
}

pub fn emit_call_arg(param: &TemplateParam) -> ::askama::Result<String> {
    Ok(param.name.clone())
}

fn emit_missing_arg(w: &mut CodeWriter, idx1: usize, name: &str) {
    w.push_line(format!(
        "if argc < {idx1} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}",
        idx1 = idx1,
        name = name
    ));
}

fn emit_argv_v(idx0: usize) -> String {
    format!("unsafe {{ *argv.add({idx0}) }}", idx0 = idx0)
}

fn emit_argv_v_expr(idx0_expr: &str) -> String {
    format!("unsafe {{ *argv.add({idx0}) }}", idx0 = idx0_expr)
}

fn emit_argv_v_let(w: &mut CodeWriter, idx0: usize) {
    // The extracted JSValue may only be needed for type checks/conversions.
    w.push_line(format!("let _v = unsafe {{ *argv.add({idx0}) }};", idx0 = idx0));
}

fn emit_check_is_string_expr(w: &mut CodeWriter, value_expr: &str, err_expr: &str) {
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsString(ctx, {v}) }} == 0 {{ return js_throw_type_error(ctx, {err}); }}",
        v = value_expr,
        err = err_expr
    ));
}

fn emit_check_is_number_expr(w: &mut CodeWriter, value_expr: &str, err_expr: &str) {
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, {v}) }} == 0 {{ return js_throw_type_error(ctx, {err}); }}",
        v = value_expr,
        err = err_expr
    ));
}


fn emit_to_i32_expr(w: &mut CodeWriter, value_expr: &str, out_name: &str, err_expr: &str) {
    w.push_line(format!("let mut {out}: i32 = 0;", out = out_name));
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToInt32(ctx, &mut {out} as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        v = value_expr,
        err = err_expr
    ));
}

fn emit_to_f64_expr(w: &mut CodeWriter, value_expr: &str, out_name: &str, err_expr: &str) {
    w.push_line(format!("let mut {out}: f64 = 0.0;", out = out_name));
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut {out} as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        v = value_expr,
        err = err_expr
    ));
}


fn emit_to_cstring_ptr_expr(w: &mut CodeWriter, value_expr: &str, name: &str, err_expr: &str) {
    w.push_line("use std::os::raw::c_char;");
    w.push_line(format!(
        "let mut {name}_buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf {{ buf: [0u8; 5] }};",
        name = name
    ));
    w.push_line(format!(
        "let {name}_ptr = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, {v}, &mut {name}_buf as *mut _) }};",
        name = name,
        v = value_expr
    ));
    w.push_line(format!(
        "if {name}_ptr.is_null() {{ return js_throw_type_error(ctx, {err}); }}",
        name = name,
        err = err_expr
    ));
    w.push_line(format!("let {name}: *const c_char = {name}_ptr;", name = name));
}


fn emit_extract_bool_expr(w: &mut CodeWriter, value_expr: &str, name: &str, err_expr: &str) {
    w.push_line(format!(
        "let {name}_v: u32 = {v} as u32;",
        name = name,
        v = value_expr
    ));
    w.push_line(format!(
        "if ({name}_v & ((1 << mquickjs_rs::mquickjs_ffi::JS_TAG_SPECIAL_BITS) - 1)) != 3 {{ return js_throw_type_error(ctx, {err}); }}",
        name = name,
        err = err_expr
    ));
    w.push_line(format!("let {name}: bool = {name}_v != 3;", name = name));
}


fn emit_single_param_extract(
    name: &str,
    ty: &Type,
    _file_mode: FileMode,
    idx0: usize,
    idx1: usize,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::String => {
            emit_missing_arg(&mut w, idx1, name);
            emit_argv_v_let(&mut w, idx0);
            let err = format!("invalid string argument: {name}");
            emit_check_is_string_expr(&mut w, "_v", &format!("\"{}\"", err));
            emit_to_cstring_ptr_expr(&mut w, "_v", "ptr", &format!("\"{}\"", err));
            w.push_line(format!("let {name}: *const c_char = ptr;", name = name));
            w.push_line(format!("let _ = {name};", name = name));
        }
        Type::Int => {
            emit_missing_arg(&mut w, idx1, name);
            emit_argv_v_let(&mut w, idx0);
            let err = format!("invalid int argument: {name}");
            emit_check_is_number_expr(&mut w, "_v", &format!("\"{}\"", err));
            emit_to_i32_expr(&mut w, "_v", name, &format!("\"{}\"", err));
        }
        Type::Bool => {
            emit_missing_arg(&mut w, idx1, name);
            emit_argv_v_let(&mut w, idx0);
            let err = format!("invalid bool argument: {name}");
            emit_extract_bool_expr(&mut w, "_v", name, &format!("\"{}\"", err));
        }
        Type::Double => {
            emit_missing_arg(&mut w, idx1, name);
            emit_argv_v_let(&mut w, idx0);
            let err = format!("invalid double argument: {name}");
            emit_check_is_number_expr(&mut w, "_v", &format!("\"{}\"", err));
            emit_to_f64_expr(&mut w, "_v", name, &format!("\"{}\"", err));
        }
        Type::Any => {
            emit_missing_arg(&mut w, idx1, name);
            w.push_line(format!(
                "let {name}: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value> = scope.value({v});",
                name = name,
                v = emit_argv_v(idx0)
            ));
        }

        _ => {
            w.push_line(format!(
                "compile_error!(\"v1 glue: unsupported parameter type for {name}\");",
                name = name
            ));
        }
    }

    Ok(w.into_string())
}

fn emit_varargs_loop_header(w: &mut CodeWriter, start_idx0: usize, with_rel: bool) {
    w.push_line(format!(
        "for i in {start}..(argc as usize) {{",
        start = start_idx0
    ));
    w.indent();
    if with_rel {
        w.push_line(format!("let rel = i - {start};", start = start_idx0));
    }
}

fn emit_varargs_collect(
    name: &str,
    ty: &Type,
    file_mode: FileMode,
    start_idx0: usize,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::String => {
            w.push_line(format!(
                "let mut {name}: Vec<*const c_char> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid string argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_string_expr(&mut w, "v", &err_expr);
            emit_to_cstring_ptr_expr(&mut w, "v", "ptr", &err_expr);
            w.push_line(format!("{name}.push(ptr);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::Int => {
            w.push_line(format!("let mut {name}: Vec<i32> = Vec::new();", name = name));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid int argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_i32_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::Bool => {
            w.push_line(format!("let mut {name}: Vec<bool> = Vec::new();", name = name));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid bool argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_extract_bool_expr(&mut w, "v as u32", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::Double => {
            w.push_line(format!("let mut {name}: Vec<f64> = Vec::new();", name = name));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid double argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_f64_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::Any => {
            let _ = file_mode;
            w.push_line(format!(
                "let mut {name}: Vec<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, false);
            w.push_line(format!(
                "{name}.push(scope.value({v}));",
                name = name,
                v = emit_argv_v_expr("i")
            ));
            w.dedent();
            w.push_line("}");
        }
        _ => {
            w.push_line(format!(
                "compile_error!(\"v1 glue: unsupported varargs type for {name}\");",
                name = name
            ));
        }
    }

    Ok(w.into_string())
}

