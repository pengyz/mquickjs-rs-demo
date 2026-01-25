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

pub fn rust_ident(name: &str) -> ::askama::Result<String> {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if out.is_empty() {
        out.push('_');
    }

    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }

    // Keep list minimal; extend when needed.
    const KW: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];

    if KW.iter().any(|&kw| kw == out) {
        out.push('_');
    }

    Ok(out)
}

pub fn rust_type_from_idl(idl_type: &Type) -> Result<String, askama::Error> {
    let rust_type = match idl_type {
        Type::Bool => "bool".to_string(),
        Type::I32 => "i32".to_string(),
        Type::I64 => "i64".to_string(),
        Type::F32 => "f32".to_string(),
        Type::F64 => "f64".to_string(),
        Type::String => "String".to_string(),
        Type::Void => "()".to_string(),

        // Optional(T) at Rust boundary.
        Type::Optional(inner) => format!("Option<{}>", rust_type_from_idl(inner)?),

        // `any` at Rust boundary:
        // - Keep representation consistent across file modes.
        // - `FileMode::Strict` only limits where `any` may appear; it must not change the ABI.
        //
        // any param is a borrowed view.
        // NOTE: params must NOT expose a free `'ctx` in API traits; methods that need to bind
        // lifetime to `Env<'ctx>` must do it at template level.
        Type::Any => {
            "mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>".to_string()
        }

        // For class refs, treat them as trait objects at Rust boundary.
        Type::ClassRef(name) => format!("Box<dyn crate::api::{}Class>", name),

        // Custom types are not supported as typed returns/params in v1.
        // They should be lowered to `any` by higher-level generator logic if needed.
        Type::Custom(_name) => {
            return Err(askama::Error::Custom(
                "v1 rust_type_from_idl: unsupported Custom named type".into(),
            ))
        }

        Type::Union(_types) => {
            return Err(askama::Error::Custom(
                "union rust type generation is not implemented yet".into(),
            ));
        }

        Type::Array(inner) => {
            format!("Vec<{}>", rust_type_from_idl(inner)?)
        }

        Type::Map(key, value) => {
            format!(
                "std::collections::HashMap<{}, {}>",
                rust_type_from_idl(key)?,
                rust_type_from_idl(value)?
            )
        }

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
        Type::I32 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, {value}) }}",
                value = value_expr
            ));
        }
        Type::I64 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt64(ctx, {value}) }}",
                value = value_expr
            ));
        }
        Type::F64 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, {value}) }}",
                value = value_expr
            ));
        }
        Type::F32 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, ({value}) as f64) }}",
                value = value_expr
            ));
        }
        Type::String => {
            w.push_line(format!(
                "let cstr = CString::new({value}.as_str()).unwrap_or_else(|_| CString::new(\"\").unwrap());",
                value = value_expr
            ));
            w.push_line("let v = unsafe { mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) };");
            w.push_line("v");
        }
        Type::Map(key_ty, value_ty) => {
            // v1: map is represented as a plain JS object.
            // NOTE: key is stored as JS property name string.
            w.push_line("let obj = unsafe { mquickjs_rs::mquickjs_ffi::JS_NewObject(ctx) };".to_string());
            w.push_line(format!("for (k, v) in {value}.iter() {{", value = value_expr));
            w.indent();

            // key -> String
            match key_ty.as_ref() {
                Type::String => {
                    w.push_line("let k_str: &str = k.as_str();".to_string());
                }
                Type::Bool | Type::I32 | Type::I64 | Type::F32 | Type::F64 => {
                    w.push_line("let k_str = k.to_string();".to_string());
                }
                _ => {
                    w.push_line(
                        "compile_error!(\"v1 glue: map key type must be primitive\");".to_string(),
                    );
                    w.push_line("let k_str = \"\".to_string();".to_string());
                }
            }

            w.push_line("let k_cstr = CString::new(k_str).unwrap_or_else(|_| CString::new(\"\").unwrap());".to_string());

            let value_to_js = emit_value_to_js(value_ty, "v")?;
            // emit_value_to_js can generate multiple statements.
            // If it ends with an expression (e.g. `v`), we must not emit it as a standalone statement.
            let mut lines: Vec<&str> = value_to_js.lines().collect();
            if let Some(&last) = lines.last() {
                let trimmed = last.trim();
                if trimmed == "v" || trimmed == "obj" {
                    lines.pop();
                }
            }
            for line in lines {
                w.push_line(line.to_string());
            }
            w.push_line("let v_js: JSValue = v;".to_string());

            w.push_line(
                "unsafe { mquickjs_rs::mquickjs_ffi::JS_SetPropertyStr(ctx, obj, k_cstr.as_ptr(), v_js) };"
                    .to_string(),
            );

            w.dedent();
            w.push_line("}".to_string());
            w.push_line("obj".to_string());
        }
        other => {
            return Err(askama::Error::Custom(
                format!("v1 glue: unsupported value conversion: {other:?}").into(),
            ));
        }
    }

    Ok(w.into_string())
}

fn emit_match_value_expr(scrutinee_expr: &str, arms: Vec<String>) -> String {
    // Generate a single expression string to avoid brace drift across multi-line emission.
    // NOTE: every arm is force-suffixed with `,`.
    let mut out = String::new();
    out.push_str("match ");
    out.push_str(scrutinee_expr);
    out.push_str(" {");

    for mut arm in arms {
        if !arm.trim_end().ends_with(',') {
            arm.push(',');
        }
        out.push(' ');
        out.push_str(&arm);
    }

    out.push_str(" }");
    out
}

fn emit_match_return(w: &mut CodeWriter, scrutinee_expr: &str, arms: Vec<String>) {
    w.push_line(format!(
        "return {};",
        emit_match_value_expr(scrutinee_expr, arms)
    ));
}

pub fn emit_return_convert_typed(
    result_rust_ty: &str,
    return_type: &Type,
    result_name: &str,
) -> ::askama::Result<String> {
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
        Type::I32 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, {result_name}) }}",
                result_name = result_name
            ));
        }
        Type::I64 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt64(ctx, {result_name}) }}",
                result_name = result_name
            ));
        }
        Type::Bool => {
            w.push_line(format!(
                "mquickjs_rs::mquickjs_ffi::js_mkbool({result_name})",
                result_name = result_name
            ));
        }
        Type::F64 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, {result_name}) }}",
                result_name = result_name
            ));
        }
        Type::F32 => {
            w.push_line(format!(
                "unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, ({result_name}) as f64) }}",
                result_name = result_name
            ));
        }
        Type::Any => {
            // any return is a Handle<Value> at the Rust boundary.
            w.push_line(format!("{result_name}.as_raw()", result_name = result_name));
        }
        Type::Map(_, _) => {
            let value_to_js = emit_value_to_js(return_type, result_name)?;
            w.push_line(value_to_js);
        }
        Type::ClassRef(name) => {
            w.push_line(format!(
                "unsafe {{ ridl_boxed_{}_to_js(ctx, {result_name}) }}",
                crate::generator::naming::to_snake_case(name),
                result_name = result_name
            ));
        }
        Type::Union(types) => {
            // v1 union return encoding: `match` expression.
            // NOTE: enum type path is precomputed and passed via `result_rust_ty`.
            let want_string = types.iter().any(|t| matches!(t, Type::String));
            let want_i32 = types.iter().any(|t| matches!(t, Type::I32));
            let want_i64 = types.iter().any(|t| matches!(t, Type::I64));
            let want_f32 = types.iter().any(|t| matches!(t, Type::F32));
            let want_f64 = types.iter().any(|t| matches!(t, Type::F64));

            let mut arms: Vec<String> = Vec::new();

            if want_string {
                arms.push(format!(
                    "{ty}::String(s) => {{ let cstr = CString::new(s).unwrap_or_else(|_| CString::new(\"\").unwrap()); unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) }} }}",
                    ty = result_rust_ty
                ));
            }
            if want_i32 {
                arms.push(format!(
                    "{ty}::I32(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, v) }}",
                    ty = result_rust_ty
                ));
            }
            if want_i64 {
                arms.push(format!(
                    "{ty}::I64(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt64(ctx, v) }}",
                    ty = result_rust_ty
                ));
            }
            if want_f32 {
                arms.push(format!(
                    "{ty}::F32(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, (v as f64)) }}",
                    ty = result_rust_ty
                ));
            }
            if want_f64 {
                arms.push(format!(
                    "{ty}::F64(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, v) }}",
                    ty = result_rust_ty
                ));
            }

            emit_match_return(&mut w, result_name, arms);
        }
        Type::Optional(inner) => {
            let mut cur: &Type = inner;
            while let Type::Group(g) = cur {
                cur = g;
            }

            match cur {
                Type::ClassRef(name) => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line("    Some(v) => {".to_string());
                    w.push_line(format!(
                        "        unsafe {{ ridl_boxed_{}_to_js(ctx, v) }}",
                        crate::generator::naming::to_snake_case(name)
                    ));
                    w.push_line("    }".to_string());
                    w.push_line("}".to_string());
                }
                Type::String => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line("    Some(s) => {".to_string());
                    w.push_line(
                        "        let cstr = CString::new(s).unwrap_or_else(|_| CString::new(\"\").unwrap());"
                            .to_string(),
                    );
                    w.push_line("        unsafe { mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) }".to_string());
                    w.push_line("    }".to_string());
                    w.push_line("}".to_string());
                }
                Type::I32 => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line(
                        "    Some(v) => unsafe { mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, v) },"
                            .to_string(),
                    );
                    w.push_line("}".to_string());
                }
                Type::I64 => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line(
                        "    Some(v) => unsafe { mquickjs_rs::mquickjs_ffi::JS_NewInt64(ctx, v) },"
                            .to_string(),
                    );
                    w.push_line("}".to_string());
                }
                Type::Bool => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line(
                        "    Some(v) => mquickjs_rs::mquickjs_ffi::js_mkbool(v),".to_string(),
                    );
                    w.push_line("}".to_string());
                }
                Type::F64 => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line("    Some(v) => unsafe { mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, v) },".to_string());
                    w.push_line("}".to_string());
                }
                Type::F32 => {
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line("    Some(v) => unsafe { mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, v as f64) },".to_string());
                    w.push_line("}".to_string());
                }
                Type::Union(types) => {
                    // Optional(union) is normalized from `A | B | null` and `(A|B)?`.
                    // NOTE: `result_rust_ty` is `Option<Enum>`, so we need the inner enum path.
                    let enum_ty = result_rust_ty
                        .trim_start_matches("Option<")
                        .trim_end_matches('>');

                    let want_string = types.iter().any(|t| matches!(t, Type::String));
                    let want_i32 = types.iter().any(|t| matches!(t, Type::I32));
                    let want_i64 = types.iter().any(|t| matches!(t, Type::I64));
                    let want_f32 = types.iter().any(|t| matches!(t, Type::F32));
                    let want_f64 = types.iter().any(|t| matches!(t, Type::F64));

                    let mut inner_arms: Vec<String> = Vec::new();
                    if want_string {
                        inner_arms.push(format!(
                            "{ty}::String(s) => {{ let cstr = CString::new(s).unwrap_or_else(|_| CString::new(\"\").unwrap()); unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewString(ctx, cstr.as_ptr()) }} }}",
                            ty = enum_ty
                        ));
                    }
                    if want_i32 {
                        inner_arms.push(format!(
                            "{ty}::I32(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt32(ctx, v) }}",
                            ty = enum_ty
                        ));
                    }
                    if want_i64 {
                        inner_arms.push(format!(
                            "{ty}::I64(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewInt64(ctx, v) }}",
                            ty = enum_ty
                        ));
                    }
                    if want_f32 {
                        inner_arms.push(format!(
                            "{ty}::F32(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, (v as f64)) }}",
                            ty = enum_ty
                        ));
                    }
                    if want_f64 {
                        inner_arms.push(format!(
                            "{ty}::F64(v) => unsafe {{ mquickjs_rs::mquickjs_ffi::JS_NewFloat64(ctx, v) }}",
                            ty = enum_ty
                        ));
                    }

                    let mut outer_arms: Vec<String> = Vec::new();
                    outer_arms.push("None => mquickjs_rs::mquickjs_ffi::JS_NULL".to_string());
                    outer_arms.push(format!(
                        "Some(u) => {{ {} }}",
                        emit_match_value_expr("u", inner_arms)
                    ));

                    emit_match_return(&mut w, result_name, outer_arms);
                }
                // ClassRef is handled at the top-level return_type match.
                Type::Any => {
                    // Optional(any) is represented as Option<ReturnAny> at Rust boundary.
                    // None => null; Some(v) => pin at the native->JS return boundary.
                    w.push_line(format!("match {result_name} {{", result_name = result_name));
                    w.push_line("    None => mquickjs_rs::mquickjs_ffi::JS_NULL,".to_string());
                    w.push_line("    Some(v) => env.pin_return(v),".to_string());
                    w.push_line("}".to_string());
                }
                Type::Map(_, _) => {
                    let value_to_js = emit_value_to_js(cur, result_name)?;
                    w.push_line(value_to_js);
                }
                other => {
                    return Err(askama::Error::Custom(
                        format!("v1 glue: unsupported return type: Optional({other:?})").into(),
                    ));
                }
            }
        }
        other => {
            return Err(askama::Error::Custom(
                format!("v1 glue: unsupported return type: {other:?}").into(),
            ));
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

pub fn methods_total_filter(
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
            w.push_line(
                "let v0: bool = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToBool(ctx, v0) } != 0;"
                    .to_string(),
            );
        }
        Type::I32 => {
            emit_check_is_number_expr(&mut w, "v0", "\"arg1: expected number\"");
            // Avoid shadowing the JSValue `v0`.
            emit_to_i32_expr(&mut w, "v0", "out0", "\"arg1: failed to convert to i32\"");
            w.push_line("let v0: i32 = out0;".to_string());
        }
        Type::I64 => {
            emit_check_is_number_expr(&mut w, "v0", "\"arg1: expected number\"");
            emit_to_i64_expr(&mut w, "v0", "out0", "\"arg1: failed to convert to i64\"");
            w.push_line("let v0: i64 = out0;".to_string());
        }
        Type::F64 | Type::F32 => {
            emit_check_is_number_expr(&mut w, "v0", "\"arg1: expected number\"");
            emit_to_f64_expr(&mut w, "v0", "v0", "\"arg1: failed to convert to number\"");
            if matches!(prop.property_type, Type::F32) {
                w.push_line("let v0: f32 = v0 as f32;".to_string());
            }
        }
        Type::String => {
            emit_check_is_string_expr(&mut w, "v0", "\"arg1: expected string\"");
            // Convert to C string pointer; in this mquickjs fork, JS_ToCString returns a borrowed pointer.
            w.push_line(
                "let mut buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf { buf: [0u8; 5] };"
                    .to_string(),
            );
            w.push_line("let cptr = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v0, &mut buf as *mut _) };".to_string());
            w.push_line("if cptr.is_null() { return js_throw_type_error(ctx, \"arg1: failed to convert to string\"); }".to_string());
            w.push_line("let v0: *const core::ffi::c_char = cptr;".to_string());
        }
        _ => {
            w.push_line(
                "return js_throw_type_error(ctx, \"setter: unsupported property type\");"
                    .to_string(),
            );
        }
    }

    Ok(w.into_string())
}

pub fn emit_param_extract(
    param: &TemplateParam,
    idx0: &usize,
    idx1: &usize,
    module_name_normalized: &str,
) -> ::askama::Result<String> {
    let raw = if param.variadic {
        emit_varargs_collect(&param.rust_name, &param.ty, param.file_mode, *idx0)?
    } else if let Type::Optional(inner) = &param.ty {
        let mut w = CodeWriter::new();
        emit_missing_arg(&mut w, *idx1, &param.rust_name);
        emit_argv_v_let(&mut w, *idx0);

        // V1: Optional(T) parameter decoding
        // - null/undefined => None
        // - otherwise decode as T (no implicit type conversions)
        w.push_line(
            "let __ridl_tag = mquickjs_rs::mquickjs_ffi::js_value_special_tag(v);".to_string(),
        );
        let opt_inner_ty = param
            .rust_ty
            .strip_prefix("Option<")
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(param.rust_ty.as_str());

        w.push_line(format!(
            "let mut __ridl_opt_{name}: Option<{ty}> = None;",
            name = param.rust_name,
            ty = opt_inner_ty
        ));

        w.push_line(format!(
            "if __ridl_tag == (mquickjs_rs::mquickjs_ffi::JS_TAG_NULL as u32) || __ridl_tag == (mquickjs_rs::mquickjs_ffi::JS_TAG_UNDEFINED as u32) {{"
        ));
        w.push_line(format!("__ridl_opt_{name} = None;", name = param.rust_name));
        w.push_line("} else {".to_string());

        // Decode inner into a local temp, then wrap Some(...)
        let inner_name = format!("{name}_inner", name = param.rust_name);

        // Reuse the already extracted JSValue `v` for optional inner decoding.
        // (Do not emit another `let v = ...` shadowing; it would also lose the original JSValue.)
        if matches!(inner.as_ref(), Type::Union(_)) {
            // For Optional(Union), decode from the already-extracted `v`.
            let inner_extract =
                emit_union_param_extract_from_jsvalue(&inner_name, inner.as_ref(), opt_inner_ty)?;
            for line in inner_extract.lines() {
                w.push_line(line.to_string());
            }
        } else {
            // For Optional(T), decode from the already-extracted `v`.
            let inner_extract = emit_single_param_extract_from_jsvalue(
                &inner_name,
                inner.as_ref(),
                module_name_normalized,
            )?;
            for line in inner_extract.lines() {
                w.push_line(line.to_string());
            }
        }

        w.push_line(format!(
            "    __ridl_opt_{name} = Some({inner_name});",
            name = param.rust_name,
            inner_name = inner_name
        ));
        w.push_line("}".to_string());

        // Optional(T) decoding always yields `Option<Inner>`.
        // For union types, rust_ty might have been overridden to `Enum` or `Option<Enum>`;
        // here we must bind the final param as `Option<Enum>`.
        let final_rust_ty = if param.rust_ty.starts_with("Option<") {
            param.rust_ty.clone()
        } else {
            format!("Option<{}>", param.rust_ty)
        };

        w.push_line(format!(
            "let {name}: {ty} = __ridl_opt_{name};",
            name = param.rust_name,
            ty = final_rust_ty
        ));

        w.into_string()
    } else if matches!(&param.ty, Type::Union(_)) {
        emit_union_param_extract(
            &param.rust_name,
            &param.ty,
            &param.rust_ty,
            param.file_mode,
            *idx0,
            *idx1,
        )?
    } else {
        emit_single_param_extract(
            &param.rust_name,
            &param.ty,
            param.file_mode,
            *idx0,
            *idx1,
            module_name_normalized,
        )?
    };

    // The template site uses 4-space indentation and expects this filter to emit aligned code.
    // Normalize inner emitters (legacy) by stripping one indent layer if present, then add ours.
    const INDENT: &str = "    ";
    let mut out = String::new();
    for (i, line) in raw.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line = line.trim_start_matches(|c: char| c == ' ' || c == '\t');
        out.push_str(INDENT);
        out.push_str(line);
    }
    out.push('\n');

    Ok(out)
}

fn emit_union_param_extract(
    name: &str,
    ty: &Type,
    rust_ty: &str,
    _file_mode: FileMode,
    idx0: usize,
    idx1: usize,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    emit_missing_arg(&mut w, idx1, name);
    emit_argv_v_let(&mut w, idx0);

    let inner = emit_union_param_extract_from_jsvalue(name, ty, rust_ty)?;
    for line in inner.lines() {
        w.push_line(line.to_string());
    }

    Ok(w.into_string())
}

fn emit_union_param_extract_from_jsvalue(
    name: &str,
    ty: &Type,
    rust_ty: &str,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    // Avoid shadowing the already-extracted JSValue `v`.
    let Type::Union(types) = ty else {
        w.push_line(format!(
            "compile_error!(\"v1 glue: emit_union_param_extract called for non-union {name}\");",
            name = name
        ));
        return Ok(w.into_string());
    };

    // v1: discriminable unions, try members in a fixed order for determinism.
    let want_string = types.iter().any(|t| matches!(t, Type::String));
    let want_i32 = types.iter().any(|t| matches!(t, Type::I32));
    let want_i64 = types.iter().any(|t| matches!(t, Type::I64));
    let want_f32 = types.iter().any(|t| matches!(t, Type::F32));
    let want_f64 = types.iter().any(|t| matches!(t, Type::F64));

    w.push_line(format!(
        "let {name}: {rust_ty} = match (|| -> Result<{rust_ty}, JSValue> {{",
        name = name,
        rust_ty = rust_ty
    ));
    w.indent();

    // NOTE: inside this closure, error paths must use `return Err(js_throw_type_error(...))`.

    if want_string {
        w.push_line(
            "if unsafe { mquickjs_rs::mquickjs_ffi::JS_IsString(ctx, v) } != 0 {".to_string(),
        );
        w.indent();
        // Inline string conversion so error path can return `Result::Err(...)` from this closure.
        w.push_line(
            "let mut ptr_buf = mquickjs_rs::mquickjs_ffi::JSCStringBuf { buf: [0u8; 5] };"
                .to_string(),
        );
        w.push_line("let ptr_ptr = unsafe { mquickjs_rs::mquickjs_ffi::JS_ToCString(ctx, v, &mut ptr_buf as *mut _) };".to_string());
        w.push_line(format!("if ptr_ptr.is_null() {{ return Err(js_throw_type_error(ctx, \"invalid union argument: {name}\")); }}", name = name));
        w.push_line("let ptr: *const core::ffi::c_char = ptr_ptr;".to_string());
        w.push_line("let s = unsafe { core::ffi::CStr::from_ptr(ptr) };".to_string());
        w.push_line(format!(
            "return Ok({rust_ty}::String(s.to_string_lossy().into_owned()));",
            rust_ty = rust_ty
        ));
        w.dedent();
        w.push_line("}".to_string());
    }

    if want_i32 || want_i64 || want_f32 || want_f64 {
        w.push_line(
            "if unsafe { mquickjs_rs::mquickjs_ffi::JS_IsNumber(ctx, v) } != 0 {".to_string(),
        );
        w.indent();

        if want_i32 {
            w.push_line("let mut __ridl_num: f64 = 0.0;".to_string());
            w.push_line("if unsafe { mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut __ridl_num as *mut _, v) } < 0 { return Err(js_throw_type_error(ctx, \"invalid union argument: number\")); }".to_string());
            w.push_line("if __ridl_num.is_finite() && (__ridl_num.fract() == 0.0) && (__ridl_num >= (i32::MIN as f64)) && (__ridl_num <= (i32::MAX as f64)) {".to_string());
            w.indent();
            w.push_line(format!(
                "return Ok({rust_ty}::I32(__ridl_num as i32));",
                rust_ty = rust_ty
            ));
            w.dedent();
            w.push_line("}".to_string());
        }

        if want_i64 {
            emit_to_i64_expr(&mut w, "v", "out", "\"invalid union argument: i64\"");
            w.push_line(format!(
                "return Ok({rust_ty}::I64(out));",
                rust_ty = rust_ty
            ));
        }
        if want_f32 {
            // Inline numeric conversion here: this closure returns Result, while the shared helper
            // emits `return js_throw_type_error(...)` for JSValue-returning glue.
            w.push_line("let v_raw: JSValue = v;".to_string());
            w.push_line("let mut out: f64 = 0.0;".to_string());
            w.push_line("if unsafe { mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut out as *mut _, v_raw) } < 0 { return Err(js_throw_type_error(ctx, \"invalid union argument: f32\")); }".to_string());
            w.push_line(format!(
                "return Ok({rust_ty}::F32(out as f32));",
                rust_ty = rust_ty
            ));
        }
        if want_f64 {
            w.push_line("let v_raw: JSValue = v;".to_string());
            w.push_line("let mut out: f64 = 0.0;".to_string());
            w.push_line("if unsafe { mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut out as *mut _, v_raw) } < 0 { return Err(js_throw_type_error(ctx, \"invalid union argument: f64\")); }".to_string());
            w.push_line(format!(
                "return Ok({rust_ty}::F64(out));",
                rust_ty = rust_ty
            ));
        }

        w.dedent();
        w.push_line("}".to_string());
    }

    w.push_line(format!(
        "Err(js_throw_type_error(ctx, \"invalid union argument: {name}\"))",
        name = name
    ));
    w.dedent();
    w.push_line("})() { Ok(v) => v, Err(e) => return e };".to_string());

    Ok(w.into_string())
}

pub fn emit_call_arg(param: &TemplateParam) -> ::askama::Result<String> {
    Ok(param.rust_name.clone())
}

fn emit_missing_arg(w: &mut CodeWriter, idx1: usize, name: &str) {
    w.push_line(format!(
        "if argc < {idx1} {{ return js_throw_type_error(ctx, \"missing argument: {name}\"); }}",
        idx1 = idx1,
        name = name
    ));
}

fn emit_argv_v_expr(idx0_expr: &str) -> String {
    format!("unsafe {{ *argv.add({idx0}) }}", idx0 = idx0_expr)
}

fn emit_argv_v_let(w: &mut CodeWriter, idx0: usize) {
    // The extracted JSValue may only be needed for type checks/conversions.
    // Keep the binding name stable so downstream templates can reference `v`.
    w.push_line(format!(
        "let _v = unsafe {{ *argv.add({idx0}) }};",
        idx0 = idx0
    ));
    w.push_line("let v: JSValue = _v;".to_string());
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
    // V1: RIDL is strict even in default mode. Do not normalize number->int by truncation.
    // Accept only integer numbers.
    let value_raw = format!("{value_expr}_raw");
    w.push_line(format!(
        "let {value_raw}: JSValue = {value_expr};",
        value_raw = value_raw,
        value_expr = value_expr
    ));

    // Use a stable, reserved-name-safe temp variable to avoid triggering non_snake_case
    // warnings when the source identifier is a keyword (e.g. `type_`).
    w.push_line("let mut __ridl_num: f64 = 0.0;".to_string());
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut __ridl_num as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        v = value_raw,
        err = err_expr
    ));
    w.push_line(format!(
        "if !__ridl_num.is_finite() || (__ridl_num.fract() != 0.0) {{ return js_throw_type_error(ctx, {err}); }}",
        err = err_expr
    ));

    w.push_line(format!("let mut {out}: i32 = 0;", out = out_name));
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToInt32(ctx, &mut {out} as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        v = value_raw,
        err = err_expr
    ));

    // Shadow the JSValue binding with the converted i32 for downstream use.
    // If the source name is not used after extraction, use `_`-prefixed name to avoid unused-variable warnings.
    let shadow_name = format!("_{value_expr}");
    w.push_line(format!(
        "let {shadow}: i32 = {out};",
        shadow = shadow_name,
        out = out_name
    ));
}

fn emit_to_f64_expr(w: &mut CodeWriter, value_expr: &str, out_name: &str, err_expr: &str) {
    let value_raw = format!("{value_expr}_raw");
    w.push_line(format!(
        "let {value_raw}: JSValue = {value_expr};",
        value_raw = value_raw,
        value_expr = value_expr
    ));

    w.push_line(format!("let mut {out}: f64 = 0.0;", out = out_name));
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut {out} as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        v = value_raw,
        err = err_expr
    ));

    // Shadow the JSValue binding with the converted f64 for downstream use.
    // If the source name is not used after extraction, use `_`-prefixed name to avoid unused-variable warnings.
    let shadow_name = format!("_{value_expr}");
    w.push_line(format!(
        "let {shadow}: f64 = {out};",
        shadow = shadow_name,
        out = out_name
    ));
}

fn emit_to_i64_expr(w: &mut CodeWriter, value_expr: &str, out_name: &str, err_expr: &str) {
    // V1 rule: JS number -> i64 only accepts safe integers (|n| <= 2^53-1) and must be integer.
    let value_raw = format!("{value_expr}_raw");
    w.push_line(format!(
        "let {value_raw}: JSValue = {value_expr};",
        value_raw = value_raw,
        value_expr = value_expr
    ));

    w.push_line(format!("let mut {out}: f64 = 0.0;", out = out_name));
    w.push_line(format!(
        "if unsafe {{ mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut {out} as *mut _, {v}) }} < 0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        v = value_raw,
        err = err_expr
    ));

    w.push_line(format!(
        "if !{out}.is_finite() || {out}.fract() != 0.0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        err = err_expr
    ));
    w.push_line(format!(
        "if {out}.abs() > 9007199254740991.0 {{ return js_throw_type_error(ctx, {err}); }}",
        out = out_name,
        err = err_expr
    ));

    w.push_line(format!("let {out}: i64 = {out} as i64;", out = out_name));

    let shadow_name = format!("_{value_expr}");
    w.push_line(format!(
        "let {shadow}: i64 = {out};",
        shadow = shadow_name,
        out = out_name
    ));
}

fn emit_to_cstring_ptr_expr(w: &mut CodeWriter, value_expr: &str, name: &str, err_expr: &str) {
    // c_char is already imported at top-level in generated glue; avoid re-importing it in nested blocks.
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
    w.push_line(format!(
        "let {name}: *const core::ffi::c_char = {name}_ptr;",
        name = name
    ));
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
    module_name_normalized: &str,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    emit_missing_arg(&mut w, idx1, name);
    emit_argv_v_let(&mut w, idx0);

    let inner = emit_single_param_extract_from_jsvalue(name, ty, module_name_normalized)?;
    for line in inner.lines() {
        w.push_line(line.to_string());
    }

    Ok(w.into_string())
}

fn emit_single_param_extract_from_jsvalue(
    name: &str,
    ty: &Type,
    module_name_normalized: &str,
) -> ::askama::Result<String> {
    let mut w = CodeWriter::new();

    match ty {
        Type::String => {
            let err = format!("invalid string argument: {name}");
            emit_check_is_string_expr(&mut w, "v", &format!("\"{}\"", err));
            emit_to_cstring_ptr_expr(&mut w, "v", "ptr", &format!("\"{}\"", err));

            // In this mquickjs fork, JS_ToCString returns a borrowed pointer.
            // Convert immediately into an owned Rust String, truncating at NUL.
            w.push_line("let s = unsafe { core::ffi::CStr::from_ptr(ptr) };".to_string());
            w.push_line(format!(
                "let {name}: String = s.to_string_lossy().into_owned();",
                name = name
            ));
        }
        Type::I32 => {
            let err = format!("invalid i32 argument: {name}");
            emit_check_is_number_expr(&mut w, "v", &format!("\"{}\"", err));
            emit_to_i32_expr(&mut w, "v", name, &format!("\"{}\"", err));
        }
        Type::I64 => {
            let err = format!("invalid i64 argument: {name}");
            emit_check_is_number_expr(&mut w, "v", &format!("\"{}\"", err));
            emit_to_i64_expr(&mut w, "v", name, &format!("\"{}\"", err));
        }
        Type::Bool => {
            let err = format!("invalid bool argument: {name}");
            emit_extract_bool_expr(&mut w, "v", name, &format!("\"{}\"", err));
        }
        Type::F64 => {
            let err = format!("invalid f64 argument: {name}");
            emit_check_is_number_expr(&mut w, "v", &format!("\"{}\"", err));
            emit_to_f64_expr(&mut w, "v", name, &format!("\"{}\"", err));
        }
        Type::F32 => {
            let err = format!("invalid f32 argument: {name}");
            emit_check_is_number_expr(&mut w, "v", &format!("\"{}\"", err));
            emit_to_f64_expr(&mut w, "v", name, &format!("\"{}\"", err));
            w.push_line(format!("let {name}: f32 = {name} as f32;", name = name));
        }
        Type::Any => {
            w.push_line(format!(
                "let {name}: mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value> = scope.value(v);",
                name = name
            ));
        }
        Type::Optional(inner) => {
            let mut cur: &Type = inner;
            while let Type::Group(g) = cur {
                cur = g;
            }

            match cur {
                Type::Any => {
                    // Optional(any) param decoding:
                    // - null/undefined => None
                    // - otherwise => Some(scope.value(v))
                    w.push_line(format!(
                        "let {name}: Option<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>> = if __ridl_tag == (mquickjs_rs::mquickjs_ffi::JS_TAG_NULL as u32) || __ridl_tag == (mquickjs_rs::mquickjs_ffi::JS_TAG_UNDEFINED as u32) {{ None }} else {{ Some(scope.value(v)) }};",
                        name = name
                    ));
                }
                _ => {
                    w.push_line(format!(
                        "compile_error!(\"v1 glue: unsupported parameter type for {name}\");",
                        name = name
                    ));
                }
            }
        }

        Type::Union(_types) => {
            w.push_line(format!(
                "compile_error!(\"v1 glue: union decoding must be generated via emit_param_extract (needs rust_ty) for {name}\");",
                name = name
            ));
        }

        Type::ClassRef(class_name) => {
            // Class parameter: expect an instance of the RIDL class (boxed trait object stored in opaque).
            // Contract: object was created by ridl_boxed_*_to_js (or constructor), which stores
            // a `*mut Box<dyn Trait>` in opaque.
            let class_snake = crate::generator::naming::to_snake_case(class_name);
            let class_upper = crate::generator::naming::to_upper_camel_case(class_name);
            // ridl_js_class_id.rs uses the normalized module name uppercased, then the class name.
            let class_id_const = format!(
                "JS_CLASS_{}_{}",
                module_name_normalized.to_ascii_uppercase(),
                class_upper.to_ascii_uppercase()
            );

            let err_invalid = format!("invalid class argument: {name}");

            // Only objects can have class_id/opaque.
            // We avoid relying on C macro tags (bindgen may not expose them).
            w.push_line(format!(
                "let __ridl_cid_{name} = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_GetClassID(ctx, v) }};",
                name = name
            ));
            w.push_line(format!(
                "if __ridl_cid_{name} != mquickjs_rs::ridl_js_class_id::{cid} {{ return js_throw_type_error(ctx, \"{err}\"); }}",
                name = name,
                cid = class_id_const,
                err = err_invalid
            ));

            // Opaque layout: the JS object stores a pointer to a `Box<dyn Trait>`.
            w.push_line(format!(
                "let __ridl_ptr_{name} = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_GetOpaque(ctx, v) }} as *mut Box<dyn crate::api::{class}Class>;",
                name = name,
                class = class_name
            ));
            w.push_line(format!(
                "if __ridl_ptr_{name}.is_null() {{ return js_throw_type_error(ctx, \"missing opaque\"); }}",
                name = name
            ));
            w.push_line(format!(
                "let {name}: Box<dyn crate::api::{class}Class> = unsafe {{ *Box::from_raw(__ridl_ptr_{name}) }};",
                name = name,
                class = class_name
            ));
            w.push_line(
                "unsafe { mquickjs_rs::mquickjs_ffi::JS_SetOpaque(ctx, v, core::ptr::null_mut()) };"
                    .to_string(),
            );
            w.push_line(format!("let _ = \"{class_snake}\";"));
        }

        Type::Map(key_ty, value_ty) => {
            let map_rust_ty = rust_type_from_idl(ty)?;
            let key_rust_ty = rust_type_from_idl(key_ty)?;

            // std::collections::HashMap requires K: Eq + Hash. f32/f64 do not implement these.
            // For now we only support primitive keys that satisfy Eq+Hash.
            // (Validator currently allows f32/f64; we reject them here with a hard error.)
            if matches!(key_ty.as_ref(), Type::F32 | Type::F64) {
                return Err(askama::Error::Custom(
                    "map key f32/f64 is not supported: HashMap requires Eq+Hash"
                        .to_string()
                        .into(),
                ));
            }

            w.push_line("let v0 = v;".to_string());
            // We treat map as a plain object with string keys.
            // mquickjs C API exposes JS_GetClassID/JS_CLASS_OBJECT but not JS_IsObject.
            w.push_line(
                "if v0 == mquickjs_rs::mquickjs_ffi::JS_NULL || unsafe { mquickjs_rs::mquickjs_ffi::JS_GetClassID(ctx, v0) } != mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_OBJECT as i32 { return js_throw_type_error(ctx, \"map: expected object\"); }"
                    .to_string(),
            );

            // keys_arr = Object.keys(v0)
            w.push_line("let global = unsafe { mquickjs_rs::mquickjs_ffi::JS_GetGlobalObject(ctx) };".to_string());
            w.push_line("use std::ffi::CString;".to_string());
            w.push_line("let __ridl_prop_object = CString::new(\"Object\").unwrap();".to_string());
            w.push_line("let __ridl_prop_keys = CString::new(\"keys\").unwrap();".to_string());
            w.push_line("let obj_ctor = unsafe { mquickjs_rs::mquickjs_ffi::JS_GetPropertyStr(ctx, global, __ridl_prop_object.as_ptr()) };".to_string());
            w.push_line("let keys_fn = unsafe { mquickjs_rs::mquickjs_ffi::JS_GetPropertyStr(ctx, obj_ctor, __ridl_prop_keys.as_ptr()) };".to_string());
            w.push_line("unsafe { mquickjs_rs::mquickjs_ffi::JS_PushArg(ctx, v0); }".to_string());
            w.push_line("unsafe { mquickjs_rs::mquickjs_ffi::JS_PushArg(ctx, keys_fn); }".to_string());
            w.push_line("unsafe { mquickjs_rs::mquickjs_ffi::JS_PushArg(ctx, obj_ctor); }".to_string());
            w.push_line("let keys_arr = unsafe { mquickjs_rs::mquickjs_ffi::JS_Call(ctx, 1) };".to_string());
            w.push_line("if mquickjs_rs::mquickjs_ffi::js_value_special_tag(keys_arr) == (mquickjs_rs::mquickjs_ffi::JS_TAG_EXCEPTION as u32) { return keys_arr; }".to_string());

            w.push_line(format!(
                "let mut {name}_map: {map_rust_ty} = std::collections::HashMap::new();",
                name = name,
                map_rust_ty = map_rust_ty
            ));

            // len = keys_arr.length
            w.push_line("let __ridl_prop_length = CString::new(\"length\").unwrap();".to_string());
            w.push_line(
                "let len_val = unsafe { mquickjs_rs::mquickjs_ffi::JS_GetPropertyStr(ctx, keys_arr, __ridl_prop_length.as_ptr()) };"
                    .to_string(),
            );
            w.push_line("let mut len_num: f64 = 0.0;".to_string());
            w.push_line(
                "if unsafe { mquickjs_rs::mquickjs_ffi::JS_ToNumber(ctx, &mut len_num as *mut _, len_val) } < 0 { return js_throw_type_error(ctx, \"map: failed to read keys length\"); }"
                    .to_string(),
            );
            w.push_line("let len: i32 = len_num as i32;".to_string());

            w.push_line("for i in 0..len {".to_string());
            w.indent();

            w.push_line(
                "let key_js = unsafe { mquickjs_rs::mquickjs_ffi::JS_GetPropertyUint32(ctx, keys_arr, i as u32) };"
                    .to_string(),
            );
            emit_to_cstring_ptr_expr(
                &mut w,
                "key_js",
                &format!("{name}_kstr"),
                "\"map: invalid key string\"",
            );
            w.push_line(format!(
                "let {name}_kstr_s = unsafe {{ core::ffi::CStr::from_ptr({name}_kstr) }}.to_string_lossy().into_owned();",
                name = name
            ));

            w.push_line(format!("let {name}_k: {key_rust_ty} = ", name = name, key_rust_ty = key_rust_ty));
            match key_ty.as_ref() {
                Type::String => {
                    w.push_line(format!("{name}_kstr_s;", name = name));
                }
                Type::Bool => {
                    w.push_line(format!(
                        "match {name}_kstr_s.as_str() {{ \"true\" => true, \"false\" => false, _ => return js_throw_type_error(ctx, \"map<bool,_>: invalid key\") }};",
                        name = name
                    ));
                }
                Type::I32 => {
                    w.push_line(format!(
                        "match {name}_kstr_s.parse::<i32>() {{ Ok(v) => v, Err(_) => return js_throw_type_error(ctx, \"map<i32,_>: invalid key\") }};",
                        name = name
                    ));
                }
                Type::I64 => {
                    w.push_line(format!(
                        "match {name}_kstr_s.parse::<i64>() {{ Ok(v) => v, Err(_) => return js_throw_type_error(ctx, \"map<i64,_>: invalid key\") }};",
                        name = name
                    ));
                }
                Type::F64 => {
                    w.push_line(format!(
                        "match {name}_kstr_s.parse::<f64>() {{ Ok(v) if v.is_finite() => if v == 0.0 {{ 0.0 }} else {{ v }}, _ => return js_throw_type_error(ctx, \"map<f64,_>: invalid key\") }};",
                        name = name
                    ));
                }
                Type::F32 => {
                    w.push_line(format!(
                        "match {name}_kstr_s.parse::<f64>() {{ Ok(v) if v.is_finite() => {{ let v = if v == 0.0 {{ 0.0 }} else {{ v }}; v as f32 }}, _ => return js_throw_type_error(ctx, \"map<f32,_>: invalid key\") }};",
                        name = name
                    ));
                }
                _ => {
                    w.push_line("return js_throw_type_error(ctx, \"map: unsupported key type\");".to_string());
                }
            }

            w.push_line(format!(
                "let {name}_prop_val = unsafe {{ mquickjs_rs::mquickjs_ffi::JS_GetPropertyStr(ctx, v0, {name}_kstr) }};",
                name = name
            ));

            // Decode value from the property JSValue.
            // emit_single_param_extract_from_jsvalue expects the source JSValue to be named `v`.
            // Keep the HashMap variable name `{name}` intact.
            w.push_line(format!("let v = {name}_prop_val;", name = name));
            let inner_extract = emit_single_param_extract_from_jsvalue(
                &format!("{name}_val"),
                value_ty,
                module_name_normalized,
            )?;
            for line in inner_extract.lines() {
                w.push_line(line.to_string());
            }

            w.push_line(format!("{name}_map.insert({name}_k, {name}_val);", name = name));

            w.dedent();
            w.push_line("}".to_string());

            w.push_line(format!("let {name} = {name}_map;", name = name));
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
                "let mut {name}: Vec<*const core::ffi::c_char> = Vec::new();",
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
        Type::I32 => {
            w.push_line(format!(
                "let mut {name}: Vec<i32> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid i32 argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_i32_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::I64 => {
            w.push_line(format!(
                "let mut {name}: Vec<i64> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid i64 argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_i64_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::Bool => {
            w.push_line(format!(
                "let mut {name}: Vec<bool> = Vec::new();",
                name = name
            ));
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
        Type::F64 => {
            w.push_line(format!(
                "let mut {name}: Vec<f64> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid f64 argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_f64_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out);", name = name));

            w.dedent();
            w.push_line("}");
        }
        Type::F32 => {
            w.push_line(format!(
                "let mut {name}: Vec<f32> = Vec::new();",
                name = name
            ));
            emit_varargs_loop_header(&mut w, start_idx0, true);
            w.push_line("let v: JSValue = unsafe { *argv.add(i) };");

            let err_expr = format!(
                "&format!(\"invalid f32 argument: {name}[{{}}]\", rel)",
                name = name
            );
            emit_check_is_number_expr(&mut w, "v", &err_expr);
            emit_to_f64_expr(&mut w, "v", "out", &err_expr);
            w.push_line(format!("{name}.push(out as f32);", name = name));

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
