// bindgen output is noisy and not actionable for this project.
#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::all
)]
pub mod mquickjs_ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    // ---- QuickJS value encoding helpers / constants ----
    // bindgen does not reliably export C macros, so we keep a small, canonical set here.

    pub const JS_NULL: JSValue = JS_VALUE_MAKE_SPECIAL(JS_TAG_NULL as u32, 0);
    pub const JS_UNDEFINED: JSValue = JS_VALUE_MAKE_SPECIAL(JS_TAG_UNDEFINED as u32, 0);
    pub const JS_FALSE: JSValue = JS_VALUE_MAKE_SPECIAL(JS_TAG_BOOL as u32, 0);
    pub const JS_TRUE: JSValue = JS_VALUE_MAKE_SPECIAL(JS_TAG_BOOL as u32, 1);

    #[inline]
    pub const fn JS_VALUE_MAKE_SPECIAL(tag: u32, v: u32) -> JSValue {
        // Matches C macro: ((tag) | ((v) << JS_TAG_SPECIAL_BITS))
        (tag | (v << (JS_TAG_SPECIAL_BITS as u32))) as JSValue
    }

    #[inline]
    pub const fn js_mkbool(v: bool) -> JSValue {
        if v {
            JS_TRUE
        } else {
            JS_FALSE
        }
    }

    #[inline]
    pub const fn js_value_special_tag(v: JSValue) -> u32 {
        (v as u32) & ((1u32 << (JS_TAG_SPECIAL_BITS as u32)) - 1)
    }

    #[inline]
    pub const fn js_is_bool(v: JSValue) -> bool {
        js_value_special_tag(v) == (JS_TAG_BOOL as u32)
    }
}

pub use context::Context;
pub use function::Function;
pub use object::Object;
pub use value::{PinnedValue, Value, ValueRef};

pub use class::{ClassId, ClassObject, Opaque};

pub mod ridl_class_id {
    include!(concat!(env!("OUT_DIR"), "/ridl_class_id.rs"));
}

pub mod context;
pub mod function;
pub mod object;
pub mod value;
pub mod class;

pub mod ridl_include;

// Note: ridl_modules are generated/aggregated by the app crate build and included there.

#[cfg(feature = "ridl-extensions")]
pub mod ridl_runtime;

#[cfg(feature = "ridl-extensions")]
pub mod ridl_ext_access;

pub fn register_extensions() {
    // Kept for API compatibility.
    // In mquickjs, C-side registration is compile-time only. The application is responsible for
    // selecting RIDL modules and linking their symbols.
}

#[cfg(feature = "ridl-extensions")]
#[macro_export]
macro_rules! ridl_bootstrap {
    () => {{
        mod __mquickjs_ridl_bootstrap {
            include!(concat!(env!("OUT_DIR"), "/ridl_bootstrap.rs"));
        }

        __mquickjs_ridl_bootstrap::ridl_initialize::initialize();
    }};
}

#[deprecated(note = "Use register_extensions() instead.")]
pub fn register_all_ridl_modules() {
    register_extensions();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    use crate::context::ContextHandle;

    #[test]
    fn test_pinned_value_survives_gc() {
        // Pin a value as a GC root and force a GC; it should remain usable.
        let mut context = Context::new(1024 * 1024).unwrap();
        let v = context.create_string("pinned").unwrap();
        let pinned = v.pin(&context);

        unsafe { mquickjs_ffi::JS_GC(context.ctx) };

        let s = context.get_string(pinned.as_ref()).unwrap();
        assert_eq!(s, "pinned");
    }

    #[test]
    fn test_tls_current_context_handle_nested() {
        let mut ctx = Context::new(1024 * 1024).unwrap();

        assert!(ContextHandle::current().is_none());

        let h1 = ctx.handle();
        let _g1 = h1.enter_current();
        let cur1 = ContextHandle::current().unwrap();
        assert_eq!(cur1.ctx, h1.ctx);

        let h2 = ctx.handle();
        let _g2 = h2.enter_current();
        let cur2 = ContextHandle::current().unwrap();
        assert_eq!(cur2.ctx, h2.ctx);

        drop(_g2);
        let cur3 = ContextHandle::current().unwrap();
        assert_eq!(cur3.ctx, h1.ctx);

        drop(_g1);
        assert!(ContextHandle::current().is_none());
    }

    #[test]
    fn test_create_context() {
        let context = Context::new(1024 * 1024);
        assert!(context.is_ok());
        let runtime = context.unwrap();
        assert!(!runtime.ctx.is_null());
    }

    #[test]
    fn test_eval_simple_expression() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");
    }

    #[test]
    fn test_eval_string() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval(r#""Hello, " + "World!""#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_eval_with_variables() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("var a = 42; a;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_eval_function() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("function add(a, b) { return a + b; }; add(5, 3);");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "8");
    }

    #[test]
    fn test_eval_error() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("undefined_variable;");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn test_eval_syntax_error() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("function test() { var a = ; } test();");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn test_eval_json() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("JSON.stringify({a: 1, b: 'test'});");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\"a\":1"));
        assert!(output.contains("\"b\":\"test\""));
    }

    #[test]
    fn test_eval_array() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let result = context.eval("[1, 2, 3].join('-');");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1-2-3");
    }

    #[test]
    fn test_multiple_evals() {
        let mut context = Context::new(1024 * 1024).unwrap();

        // First evaluation
        let result1 = context.eval("var x = 10;");
        assert!(result1.is_ok());

        // Second evaluation using previous variable
        let result2 = context.eval("x * 2;");
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), "20");
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let operations = vec![
            ("2 + 2", "4"),
            ("10 - 3", "7"),
            ("4 * 5", "20"),
            ("15 / 3", "5"),
            ("2 ** 3", "8"),
            ("17 % 5", "2"),
        ];

        for (expr, expected) in operations {
            let result = context.eval(expr).unwrap();
            assert_eq!(result, expected, "Failed for expression: {}", expr);
        }
    }

    #[test]
    fn test_boolean_operations() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let operations = vec![
            ("true && false", "false"),
            ("true || false", "true"),
            ("!true", "false"),
            ("!false", "true"),
            ("5 > 3", "true"),
            ("5 < 3", "false"),
            ("5 == 5", "true"),
            ("5 != 3", "true"),
        ];

        for (expr, expected) in operations {
            let result = context.eval(expr).unwrap();
            assert_eq!(result, expected, "Failed for expression: {}", expr);
        }
    }

    #[test]
    fn test_string_operations() {
        let mut context = Context::new(1024 * 1024).unwrap();
        let operations = vec![
            (r#""hello".toUpperCase()"#, "HELLO"),
            (r#""world".length"#, "5"),
            (r#""hello".charAt(1)"#, "e"),
            (r#""hello".substring(1, 4)"#, "ell"),
        ];

        for (expr, expected) in operations {
            let result = context.eval(expr).unwrap();
            assert_eq!(result, expected, "Failed for expression: {}", expr);
        }
    }

    #[test]
    fn test_string_creation() {
        let context = Context::new(1024 * 1024).unwrap();
        let value = context.create_string("Hello, World!").unwrap();
        assert!(value.is_string(&context));

        let js_str = context.get_string(value).unwrap();
        assert_eq!(js_str, "Hello, World!");
    }

    #[test]
    fn test_value_type_checks() {
        let context = Context::new(1024 * 1024).unwrap();

        // Test string
        let c_code = CString::new("'test string'").unwrap();
        let filename = CString::new("eval.js").unwrap();
        let str_val = unsafe {
            mquickjs_ffi::JS_Eval(
                context.ctx,
                c_code.as_ptr(),
                13,
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };
        let value = ValueRef::new(str_val);
        assert!(value.is_string(&context));

        // Test number
        let c_code = CString::new("42").unwrap();
        let filename = CString::new("eval.js").unwrap();
        let num_val = unsafe {
            mquickjs_ffi::JS_Eval(
                context.ctx,
                c_code.as_ptr(),
                2,
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };
        let value = ValueRef::new(num_val);
        assert!(value.is_number(&context));

        // Test boolean
        let c_code = CString::new("true").unwrap();
        let filename = CString::new("eval.js").unwrap();
        let bool_val = unsafe {
            mquickjs_ffi::JS_Eval(
                context.ctx,
                c_code.as_ptr(),
                4,
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };
        let value = ValueRef::new(bool_val);
        assert!(value.is_bool(&context));

        // Test null
        let c_code = CString::new("null").unwrap();
        let filename = CString::new("eval.js").unwrap();
        let null_val = unsafe {
            mquickjs_ffi::JS_Eval(
                context.ctx,
                c_code.as_ptr(),
                4,
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };
        let value = ValueRef::new(null_val);
        assert!(value.is_null(&context));

        // Test undefined
        let c_code = CString::new("undefined").unwrap();
        let filename = CString::new("eval.js").unwrap();
        let undef_val = unsafe {
            mquickjs_ffi::JS_Eval(
                context.ctx,
                c_code.as_ptr(),
                9,
                filename.as_ptr(),
                mquickjs_ffi::JS_EVAL_RETVAL as i32,
            )
        };
        let value = ValueRef::new(undef_val);
        assert!(value.is_undefined(&context));
    }
}
