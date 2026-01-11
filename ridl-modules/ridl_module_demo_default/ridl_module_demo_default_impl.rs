use std::ffi::CStr;
use std::os::raw::c_char;

use mquickjs_rs::mquickjs_ffi::JSValue;

pub fn default_echo_str(s: *const c_char) -> String {
    if s.is_null() {
        return "".to_string();
    }
    unsafe { CStr::from_ptr(s).to_string_lossy().into_owned() }
}

pub fn default_add_i32(a: i32, b: i32) -> i32 {
    a + b
}

pub fn default_not_bool(v: bool) -> bool {
    !v
}

pub fn default_add_f64(a: f64, b: f64) -> f64 {
    a + b
}

pub fn default_id_any(v: JSValue) -> JSValue {
    v
}

pub fn default_void_ok() {
    // no-op
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_demo_ping(
    _ctx: *mut core::ffi::c_void,
    _this_val: JSValue,
    s: *const c_char,
) -> JSValue {
    let _ = default_echo_str(s);
    0x02 // JS_UNDEFINED
}
