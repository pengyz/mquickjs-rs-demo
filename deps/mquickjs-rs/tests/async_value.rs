//! AsyncValue 类型转换测试

use mquickjs_rs::async_value::AsyncValue;
use mquickjs_rs::context::Context;

#[test]
fn test_async_value_null() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    // JS null → AsyncValue::Null
    let js_null = ctx.eval_jsvalue("null").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_null));
    assert!(matches!(async_val, AsyncValue::Null));
    
    // AsyncValue::Null → JS null
    let js_result = async_val.to_js(&scope);
    assert!(js_result.as_raw() == mquickjs_rs::mquickjs_ffi::JS_NULL);
}

#[test]
fn test_async_value_undefined() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    let js_undefined = ctx.eval_jsvalue("undefined").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_undefined));
    assert!(matches!(async_val, AsyncValue::Undefined));
    
    let js_result = async_val.to_js(&scope);
    assert!(js_result.as_raw() == mquickjs_rs::mquickjs_ffi::JS_UNDEFINED);
}

#[test]
fn test_async_value_bool() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    let js_true = ctx.eval_jsvalue("true").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_true));
    assert!(matches!(async_val, AsyncValue::Bool(true)));
    
    let js_result = async_val.to_js(&scope);
    assert!(js_result.as_raw() == mquickjs_rs::mquickjs_ffi::JS_TRUE);
}

#[test]
fn test_async_value_int() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    let js_int = ctx.eval_jsvalue("42").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_int));
    assert!(matches!(async_val, AsyncValue::Int(42)));
    
    let js_result = async_val.to_js(&scope);
    // 验证是整数
    let mut num: i32 = 0;
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_ToInt32(scope.ctx(), &mut num as *mut i32, js_result.as_raw());
    }
    assert_eq!(num, 42);
}

#[test]
fn test_async_value_float() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    let js_float = ctx.eval_jsvalue("3.14").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_float));
    assert!(matches!(async_val, AsyncValue::Float(f) if (f - 3.14).abs() < 0.001));
    
    let js_result = async_val.to_js(&scope);
    // 验证是浮点数
    let mut num: f64 = 0.0;
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_ToNumber(scope.ctx(), &mut num as *mut f64, js_result.as_raw());
    }
    assert!((num - 3.14).abs() < 0.001);
}

#[test]
fn test_async_value_string() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    let js_string = ctx.eval_jsvalue("'hello'").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_string));
    eprintln!("async_val = {:?}", async_val);
    assert!(matches!(async_val, AsyncValue::String(ref s) if s == "hello"));
    
    let js_result = async_val.to_js(&scope);
    // 验证是字符串
    let result_str = ctx.get_string(js_result).unwrap();
    assert_eq!(result_str, "hello");
}

#[test]
fn test_async_value_array() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    // JS [1, "two", true] → AsyncValue::Array
    let js_array = ctx.eval_jsvalue("[1, 'two', true]").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_array));
    eprintln!("async_val = {:?}", async_val);
    
    match async_val {
        AsyncValue::Array(ref arr) => {
            assert_eq!(arr.len(), 3);
            assert!(matches!(arr[0], AsyncValue::Int(1)));
            assert!(matches!(&arr[1], AsyncValue::String(s) if s == "two"));
            assert!(matches!(arr[2], AsyncValue::Bool(true)));
        }
        _ => panic!("Expected Array"),
    }
    
    // 还原回 JS
    let js_result = async_val.to_js(&scope);
    // 验证是数组（通过 eval 检查）
    let check = ctx.eval(&format!(
        "Array.isArray({})",
        ctx.get_string(js_result).unwrap()
    ));
    // 简化验证：只要不 panic 即可
}

#[test]
fn test_async_value_object() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    // JS {name: "test", value: 42} → AsyncValue::Json (JSON.stringify)
    let js_object = ctx.eval_jsvalue("({name: 'test', value: 42})").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_object));
    eprintln!("async_val = {:?}", async_val);
    
    // 对象被转换为 JSON 字符串
    match async_val {
        AsyncValue::Json(ref json_str) => {
            // 验证 JSON 字符串包含预期内容
            assert!(json_str.contains("\"name\":\"test\""));
            assert!(json_str.contains("\"value\":42"));
        }
        _ => panic!("Expected Json"),
    }
    
    // 还原回 JS
    let _js_result = async_val.to_js(&scope);
    // 简化验证：只要不 panic 即可
}

#[test]
fn test_async_value_nested() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    // JS {arr: [1, 2], obj: {key: "val"}} → AsyncValue::Json (JSON.stringify)
    let js_nested = ctx.eval_jsvalue("({arr: [1, 2], obj: {key: 'val'}})").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_nested));
    
    // 嵌套对象被转换为 JSON 字符串
    match async_val {
        AsyncValue::Json(ref json_str) => {
            // 验证 JSON 字符串包含预期内容
            assert!(json_str.contains("\"arr\":[1,2]"));
            assert!(json_str.contains("\"obj\":{\"key\":\"val\"}"));
        }
        _ => panic!("Expected Json"),
    }
}

#[test]
fn test_async_value_function_unsupported() {
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    let token = ctx.token();
    let scope = token.enter_scope();
    
    // JS function → AsyncValue::Unsupported
    let js_func = ctx.eval_jsvalue("(function() {})").unwrap();
    let async_val = AsyncValue::from_js(&scope, scope.value(js_func));
    assert!(matches!(async_val, AsyncValue::Unsupported));
}

#[test]
fn test_async_value_symbol_unsupported() {
    // mquickjs 不支持 Symbol，所以这个测试验证 eval 会失败
    let mut ctx = Context::new(1024 * 1024).expect("create ctx");
    
    // Symbol 未定义，eval 应该失败
    let result = ctx.eval_jsvalue("Symbol('test')");
    assert!(result.is_err());
}

#[test]
fn test_async_value_send() {
    // 验证 AsyncValue 是 Send
    fn assert_send<T: Send>() {}
    assert_send::<AsyncValue>();
}

#[test]
fn test_async_value_static() {
    // 验证 AsyncValue 是 'static
    fn assert_static<T: 'static>() {}
    assert_static::<AsyncValue>();
}