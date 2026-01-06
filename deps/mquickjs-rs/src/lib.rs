use std::os::raw::{c_void};
use std::ffi::{CString, CStr};
use std::marker::PhantomData;

// 导入生成的绑定
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(dead_code)]
#[allow(clippy::all)]
pub mod mquickjs_ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// 定义条件宏，用于引入RIDL扩展符号
#[cfg(feature = "ridl-extensions")]
#[macro_export]
macro_rules! mquickjs_ridl_extensions {
    () => {
        include!("../ridl_symbols.rs");
    }
}

#[cfg(not(feature = "ridl-extensions"))]
#[macro_export]
macro_rules! mquickjs_ridl_extensions {
    () => {
        // 当ridl-extensions feature关闭时，宏不展开任何内容
    }
}

pub use context::Context;
pub use value::Value;
pub use object::Object;
pub use function::Function;

pub mod context;
pub mod value;
pub mod object;
pub mod function;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

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
        let mut context = Context::new(1024 * 1024).unwrap();
        let value = context.create_string("Hello, World!").unwrap();
        assert!(value.is_string(&context));
        
        let js_str = context.get_string(value).unwrap();
        assert_eq!(js_str, "Hello, World!");
    }

    #[test]
    fn test_value_type_checks() {
        let mut context = Context::new(1024 * 1024).unwrap();
        
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
        let value = Value {
            value: str_val,
            _ctx: PhantomData,
        };
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
        let value = Value {
            value: num_val,
            _ctx: PhantomData,
        };
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
        let value = Value {
            value: bool_val,
            _ctx: PhantomData,
        };
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
        let value = Value {
            value: null_val,
            _ctx: PhantomData,
        };
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
        let value = Value {
            value: undef_val,
            _ctx: PhantomData,
        };
        assert!(value.is_undefined(&context));
    }
}

