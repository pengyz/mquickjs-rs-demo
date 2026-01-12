use crate::generator::code_writer::CodeWriter;
use crate::generator::TemplateParam;
use crate::parser::ast::{PropertyModifier, Type};

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
        // NOTE: `rust_api.rs.j2` only needs basic property types today.
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),
        Type::Optional(inner) => format!("Option<{}>", rust_type_from_idl(inner)?),
        Type::Custom(name) => name.clone(),
        _ => "serde_json::Value".to_string(),
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
            w.push_line("JS_UNDEFINED");
        }
    }

    Ok(w.into_string())
}

pub fn emit_return_convert(return_type: &Type, result_name: &str) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match return_type {
        Type::Void => {
            w.push_line(format!("let _ = {result_name};", result_name = result_name));
            w.push_line("JS_UNDEFINED");
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
            w.push_line(result_name);
        }
        _ => {
            w.push_line("compile_error!(\"v1 glue: unsupported return type\");");
            w.push_line("JS_UNDEFINED");
        }
    }

    Ok(w.into_string())
}

pub fn is_readonly_prop(modifiers: &[PropertyModifier]) -> ::askama::Result<bool> {
    Ok(modifiers.contains(&PropertyModifier::ReadOnly))
}

pub fn emit_param_extract(
    param: &TemplateParam,
    idx0: &usize,
    idx1: &usize,
) -> ::askama::Result<String> {
    if param.variadic {
        emit_varargs_collect(&param.name, &param.ty, *idx0)
    } else {
        emit_single_param_extract(&param.name, &param.ty, *idx0, *idx1)
    }
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

fn emit_check_is_string(w: &mut CodeWriter, idx0: usize, err: &str) {
    emit_check_is_string_expr(w, &emit_argv_v(idx0), &format!("\"{}\"", err))
}

fn emit_check_is_number(w: &mut CodeWriter, idx0: usize, err: &str) {
    emit_check_is_number_expr(w, &emit_argv_v(idx0), &format!("\"{}\"", err))
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

fn emit_to_i32(w: &mut CodeWriter, idx0: usize, out_name: &str, err: &str) {
    emit_to_i32_expr(w, &emit_argv_v(idx0), out_name, &format!("\"{}\"", err))
}

fn emit_to_f64(w: &mut CodeWriter, idx0: usize, out_name: &str, err: &str) {
    emit_to_f64_expr(w, &emit_argv_v(idx0), out_name, &format!("\"{}\"", err))
}

fn emit_to_cstring_ptr_expr(w: &mut CodeWriter, value_expr: &str, name: &str, err_expr: &str) {
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

fn emit_to_cstring_ptr(w: &mut CodeWriter, idx0: usize, name: &str, err: &str) {
    emit_to_cstring_ptr_expr(w, &emit_argv_v(idx0), name, &format!("\"{}\"", err))
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

fn emit_extract_bool(w: &mut CodeWriter, idx0: usize, name: &str, err: &str) {
    emit_extract_bool_expr(w, &emit_argv_v(idx0), name, &format!("\"{}\"", err))
}

fn emit_single_param_extract(
    name: &str,
    ty: &Type,
    idx0: usize,
    idx1: usize,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::String => {
            emit_missing_arg(&mut w, idx1, name);
            let err = format!("invalid string argument: {name}");
            emit_check_is_string(&mut w, idx0, &err);
            emit_to_cstring_ptr(&mut w, idx0, name, &err);
        }
        Type::Int => {
            emit_missing_arg(&mut w, idx1, name);
            let err = format!("invalid int argument: {name}");
            emit_check_is_number(&mut w, idx0, &err);
            emit_to_i32(&mut w, idx0, name, &err);
        }
        Type::Bool => {
            emit_missing_arg(&mut w, idx1, name);
            let err = format!("invalid bool argument: {name}");
            emit_extract_bool(&mut w, idx0, name, &err);
        }
        Type::Double => {
            emit_missing_arg(&mut w, idx1, name);
            let err = format!("invalid double argument: {name}");
            emit_check_is_number(&mut w, idx0, &err);
            emit_to_f64(&mut w, idx0, name, &err);
        }
        Type::Any => {
            emit_missing_arg(&mut w, idx1, name);
            w.push_line(format!(
                "let {name}: JSValue = {v};",
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

fn emit_varargs_loop_header(w: &mut CodeWriter, start_idx0: usize) {
    w.push_line(format!(
        "for i in {start}..(argc as usize) {{",
        start = start_idx0
    ));
    w.indent();
    w.push_line(format!("let rel = i - {start};", start = start_idx0));
}

fn emit_varargs_collect(name: &str, ty: &Type, start_idx0: usize) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::String => {
            w.push_line(format!(
                "let mut {name}: Vec<*const c_char> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0);
            w.push_line("let v = unsafe { *argv.add(i) };");

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
            emit_varargs_loop_header(&mut w, start_idx0);
            w.push_line("let v = unsafe { *argv.add(i) };");

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
            emit_varargs_loop_header(&mut w, start_idx0);
            w.push_line("let v = unsafe { *argv.add(i) };");

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
            emit_varargs_loop_header(&mut w, start_idx0);
            w.push_line("let v = unsafe { *argv.add(i) };");

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
            w.push_line(format!("let mut {name}: Vec<JSValue> = Vec::new();", name = name));
            emit_varargs_loop_header(&mut w, start_idx0);
            w.push_line(format!(
                "{name}.push({v});",
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

