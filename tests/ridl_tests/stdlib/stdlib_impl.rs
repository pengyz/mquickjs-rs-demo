/*
 * Rust实现文件，提供stdlib.ridl中定义功能的实际实现
 * 这些函数会被stdlib_glue.c中的C函数调用
 */

use std::os::raw::{c_char, c_int};
use std::ffi::{CString, CStr};
use std::time::{SystemTime, UNIX_EPOCH};

// 从mquickjs.h导入必要的类型定义
extern "C" {
    // 从mquickjs导入必要的函数
    fn JS_NewString(ctx: *mut std::ffi::c_void, buf: *const c_char) -> u32;
    fn JS_NewFloat64(ctx: *mut std::ffi::c_void, d: f64) -> u32;
    fn JS_NewInt32(ctx: *mut std::ffi::c_void, val: i32) -> u32;
    fn JS_UNDEFINED() -> u32;
    fn JS_GetPropertyStr(ctx: *mut std::ffi::c_void, this_obj: u32, str_ptr: *const c_char) -> u32;
    fn JS_SetPropertyStr(ctx: *mut std::ffi::c_void, this_obj: u32, str_ptr: *const c_char, val: u32) -> u32;
    fn JS_NewObject(ctx: *mut std::ffi::c_void) -> u32;
    fn JS_GC(ctx: *mut std::ffi::c_void);
    fn JS_ToCStringLen(ctx: *mut std::ffi::c_void, plen: *mut usize, val: u32, buf: *mut std::ffi::c_void) -> *const c_char;
    fn JS_FreeCString(str_ptr: *const c_char);
    fn JS_IsString(ctx: *mut std::ffi::c_void, val: u32) -> c_int;
    fn JS_PrintValueF(ctx: *mut std::ffi::c_void, val: u32, flags: c_int);
    fn JS_ToInt32(ctx: *mut std::ffi::c_void, pres: *mut i32, val: u32) -> c_int;
    fn JS_ToNumber(ctx: *mut std::ffi::c_void, pres: *mut f64, val: u32) -> c_int;
    fn JS_IsFunction(ctx: *mut std::ffi::c_void, val: u32) -> c_int;
    fn JS_ThrowTypeError(ctx: *mut std::ffi::c_void, fmt: *const c_char, ...) -> u32;
    fn JS_EXCEPTION() -> u32;
    fn JS_IsException(val: u32) -> c_int;
}

// 导出C函数供C代码调用
#[no_mangle]
pub extern "C" fn rust_console_log(
    _ctx: *mut std::ffi::c_void,
    _this_val: u32,
    argc: c_int,
    args: *mut *mut c_char,
) -> u32 {
    let args_slice = unsafe { std::slice::from_raw_parts(args, argc as usize) };
    
    for (i, &arg_ptr) in args_slice.iter().enumerate() {
        if !arg_ptr.is_null() {
            if i != 0 {
                print!(" ");
            }
            
            let c_str = unsafe { CStr::from_ptr(arg_ptr) };
            if let Ok(s) = c_str.to_str() {
                print!("{}", s);
            } else {
                print!("[invalid string]");
            }
        }
    }
    println!();
    
    unsafe { JS_UNDEFINED() }
}

#[no_mangle]
pub extern "C" fn rust_console_error(
    _ctx: *mut std::ffi::c_void,
    _this_val: u32,
    argc: c_int,
    args: *mut *mut c_char,
) -> u32 {
    let args_slice = unsafe { std::slice::from_raw_parts(args, argc as usize) };
    
    for (i, &arg_ptr) in args_slice.iter().enumerate() {
        if !arg_ptr.is_null() {
            if i != 0 {
                eprint!(" ");
            }
            
            let c_str = unsafe { CStr::from_ptr(arg_ptr) };
            if let Ok(s) = c_str.to_str() {
                eprint!("{}", s);
            } else {
                eprint!("[invalid string]");
            }
        }
    }
    eprintln!();
    
    unsafe { JS_UNDEFINED() }
}

#[no_mangle]
pub extern "C" fn rust_date_now(
    _ctx: *mut std::ffi::c_void,
) -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as f64
}

#[no_mangle]
pub extern "C" fn rust_performance_now(
    _ctx: *mut std::ffi::c_void,
) -> f64 {
    // 获取当前时间作为性能时间戳
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as f64
}

#[no_mangle]
pub extern "C" fn rust_gc(
    ctx: *mut std::ffi::c_void,
) -> u32 {
    unsafe {
        JS_GC(ctx);
    }
    unsafe { JS_UNDEFINED() }
}

#[no_mangle]
pub extern "C" fn rust_load(
    ctx: *mut std::ffi::c_void,
    filename: *const c_char,
) -> u32 {
    if filename.is_null() {
        return unsafe { JS_UNDEFINED() };
    }

    let c_str = unsafe { CStr::from_ptr(filename) };
    if let Ok(filename_str) = c_str.to_str() {
        // 在实际实现中，这里会加载并执行JS文件
        // 暂时返回一个占位符值
        println!("Loading file: {}", filename_str);
    }
    
    unsafe { JS_UNDEFINED() }
}

#[no_mangle]
pub extern "C" fn rust_setTimeout(
    _ctx: *mut std::ffi::c_void,
    func: u32,
    delay: c_int,
) -> u32 {
    // 在实际实现中，这里会设置一个定时器
    // 暂时返回一个占位符值
    println!("Setting timeout with delay: {}ms", delay);
    unsafe { JS_NewInt32(_ctx, 0) }
}

#[no_mangle]
pub extern "C" fn rust_clearTimeout(
    _ctx: *mut std::ffi::c_void,
    timer_id: c_int,
) -> u32 {
    // 在实际实现中，这里会清除定时器
    // 暂时返回一个占位符值
    println!("Clearing timeout with ID: {}", timer_id);
    unsafe { JS_UNDEFINED() }
}

#[no_mangle]
pub extern "C" fn rust_say_hello(
    ctx: *mut std::ffi::c_void,
) -> u32 {
    // 返回 "Hello from Rust!" 字符串
    let hello_str = "Hello from Rust!";
    let c_str = CString::new(hello_str).unwrap();
    unsafe {
        JS_NewString(ctx, c_str.as_ptr())
    }
}

// 该模块用于与mquickjs-rs集成
// 提供从Rust调用到C实现的绑定
pub mod stdlib_bindings {
    use super::*;

    pub fn register_stdlib_functions(ctx: *mut std::ffi::c_void) -> c_int {
        // 在完整实现中，这里会调用C代码来注册模块
        // 但现在我们只保留接口定义
        0
    }
}

// 未来可能需要的模块注册接口
pub fn register_stdlib_module() {
    // 在完整实现中，这里会调用C代码来注册模块
    // 但现在我们只保留接口定义
}