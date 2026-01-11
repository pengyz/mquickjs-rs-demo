//! RIDL stdlib: Rust-side implementations.
//!
//! NOTE: This file intentionally starts minimal.
//! We are currently focusing on getting `console.log(content: string)` working
//! end-to-end via the build-time stdlib injection mechanism.

use std::ffi::CStr;
use std::os::raw::c_char;


#[no_mangle]
pub extern "C" fn rust_console_log(
    _ctx: *mut std::ffi::c_void,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    content: *const c_char,
) -> mquickjs_rs::mquickjs_ffi::JSValue {
    if !content.is_null() {
        let c_str = unsafe { CStr::from_ptr(content) };
        if let Ok(s) = c_str.to_str() {
            println!("{s}");
        } else {
            println!("[invalid utf-8]");
        }
    } else {
        println!();
    }

    0x02 // JS_UNDEFINED
}

#[no_mangle]
pub extern "C" fn rust_console_error(
    _ctx: *mut std::ffi::c_void,
    _this_val: mquickjs_rs::mquickjs_ffi::JSValue,
    content: *const c_char,
) -> mquickjs_rs::mquickjs_ffi::JSValue {
    if !content.is_null() {
        let c_str = unsafe { CStr::from_ptr(content) };
        if let Ok(s) = c_str.to_str() {
            eprintln!("{s}");
        } else {
            eprintln!("[invalid utf-8]");
        }
    } else {
        eprintln!();
    }

    0x02 // JS_UNDEFINED
}
