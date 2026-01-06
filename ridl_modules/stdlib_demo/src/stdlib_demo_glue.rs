use mquickjs_rs::mquickjs_ffi::{
    JSContext, JSValue, JS_NewString
};

use crate::stdlib_demo_impl::say_hello;

#[no_mangle]
pub unsafe extern "C" fn js_say_hello(
    ctx: *mut JSContext,
    _this_val: JSValue,
    _argc: i32,
    _argv: *mut JSValue,
) -> JSValue {
    // 调用impl.rs中的具体实现
    let result = say_hello();

    // 将Rust类型转换为JSValue并返回
    JS_NewString(ctx, result.as_ptr() as *const i8)
}