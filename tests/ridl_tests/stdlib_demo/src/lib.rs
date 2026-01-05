use std::convert::TryInto;
use std::mem;
use std::ptr;

use mquickjs_sys::{
    JSContext, JSValue, JS_Call, JS_DefinePropertyValueStr, JS_FreeValue, JS_GetPropertyStr,
    JS_NewCFunction, JS_NewObject, JS_SetPropertyStr, JS_VALUE_GET_FLOAT64, JS_VALUE_GET_TAG,
    JS_TAG_EXCEPTION, JS_TAG_STRING, JS_ToCStringLen2,
};

#[no_mangle]
pub unsafe extern "C" fn js_say_hello(
    _ctx: *mut JSContext,
    argc: i32,
    argv: *mut JSValue,
) -> JSValue {
    // We don't need any parameters for this function
    // if argc != 0 {
    //     // Handle incorrect number of arguments
    //     return mquickjs_sys::JS_ThrowTypeError(_ctx, b"say_hello expects no arguments\0".as_ptr() as *const i8);
    // }

    // Return "Hello, World!" string
    let ret = say_hello();
    JS_NewCStringLen2(_ctx, ret.as_ptr(), ret.len())
}
fn say_hello() -> String {
    // We don't need any parameters for this function
    // if argc != 0 {
    //     // Handle incorrect number of arguments
    //     return mquickjs_sys::JS_ThrowTypeError(_ctx, b"say_hello expects no arguments\0".as_ptr() as *const i8);
    // }

    // Return "Hello, World!" string
    "Hello, World!"
}