// Auto-generated Rust glue code for RIDL interfaces
#[allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use mquickjs_rs::mquickjs_ffi::{JSContext, JSValue};

// Mode-agnostic prelude (std and no_std consumers): CString/c_int/Vec/Box are
// re-exported from alloc/core by mquickjs-rs, so the generated glue never
// depends on the std prelude (bare-metal RIDL, see mquickjs-rs::glue_prelude).
use mquickjs_rs::glue_prelude::*;

#[allow(unused_imports)]
use crate::impls::*;

// Module initializer API
pub fn initialize_module() {
    __ridl_symbols::ensure_symbols();
}

// Fill per-context RIDL extension slots for this module.
// Called by the app-level aggregated ridl_context_init.
//
// This API must not reference any app crate types.
pub fn ridl_module_context_init(w: &mut dyn mquickjs_rs::ridl_runtime::RidlSlotWriter) {
    let _ = w;
}

// Erased ctx slot vtables consumed by the app-side aggregated ridl_context_init.
//
// - singleton slots:
//   - Module must provide a Rust-level creator: `crate::impls::create_<name>_singleton() -> Box<dyn Trait>`.
//   - Glue exports the C ABI creator: `#[no_mangle] extern "C" fn ridl_create_<name>_singleton() -> *mut c_void`.
//     Return value is an erased *thin* pointer produced by `Box::into_raw(Box<Box<dyn Trait>>)`.
// - class proto slots: module provides `crate::ridl_create_proto_<class>() -> <Impl as Class>::Proto`
pub static RIDL_CONSOLE_CTX_SLOT_VT: ::mquickjs_rs::ridl_runtime::RidlErasedSlotVTable =
    ::mquickjs_rs::ridl_runtime::RidlErasedSlotVTable {
        create: ridl_console_singleton_create,
        drop: ridl_console_singleton_drop,
    };

#[unsafe(no_mangle)]
pub extern "C" fn ridl_create_console_singleton() -> *mut core::ffi::c_void {
    let b: Box<dyn crate::api::ConsoleSingleton> =
        crate::impls::create_console_singleton();
    let holder: Box<Box<dyn crate::api::ConsoleSingleton>> = Box::new(b);
    Box::into_raw(holder) as *mut core::ffi::c_void
}

unsafe extern "C" fn ridl_console_singleton_create() -> *mut core::ffi::c_void {
    // The per-context slot vtable uses the exported creator symbol.
    ridl_create_console_singleton()
}

unsafe extern "C" fn ridl_console_singleton_drop(p: *mut core::ffi::c_void) {
    if !p.is_null() {
        unsafe {
            // Safety: `p` is a thin pointer produced by `Box::into_raw(Box<Box<dyn Trait>>)` in exported creator.
            let holder: Box<Box<dyn crate::api::ConsoleSingleton>> = Box::from_raw(p as *mut _);
            drop(holder);
        }
    }
}

mod __ridl_symbols {
    #[allow(unused_imports)]
    use super::*;

    // Keep glue symbols linked.
    // NOTE: we intentionally use `extern` + function pointers here to avoid depending on
    // any generated module layout (`crate::generated::...`).
    unsafe extern "C" {
        fn js_global_singleton_console_log(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue;
        fn js_global_singleton_console_error(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue;
        fn js_global_singleton_console_get_enabled(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue;
    }

    pub fn ensure_symbols() {
        let _ = js_global_singleton_console_log as unsafe extern "C" fn(*mut JSContext, *mut JSValue, c_int, *mut JSValue) -> JSValue;
        let _ = js_global_singleton_console_error as unsafe extern "C" fn(*mut JSContext, *mut JSValue, c_int, *mut JSValue) -> JSValue;
        let _ = js_global_singleton_console_get_enabled as unsafe extern "C" fn(*mut JSContext, *mut JSValue, c_int, *mut JSValue) -> JSValue;
    }
}

#[inline]
fn js_throw_type_error(ctx: *mut JSContext, msg: &str) -> JSValue {
    let cstr = CString::new(msg).unwrap_or_else(|_| CString::new("TypeError").unwrap());
    // mquickjs exposes JS_ThrowTypeError as a macro; bindings expose JS_ThrowError.
    // JS_CLASS_TYPE_ERROR is stable in this fork.
    unsafe {
        mquickjs_rs::mquickjs_ffi::JS_ThrowError(
            ctx,
            mquickjs_rs::mquickjs_ffi::JSObjectClassEnum_JS_CLASS_TYPE_ERROR,
            cstr.as_ptr(),
        )
    }
}

// Call into crate-local implementations.
// The module crate is expected to provide `crate::impls::*`.

// Glue implementations for functions

// Glue implementations for singletons
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_global_singleton_console_log(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue {
    let _ = this_val;
    // v_next (A2): per-JSContext singleton dispatch via ctx user_data -> ContextInner -> ridl_ext.
    let Some(h) = (unsafe { mquickjs_rs::context::ContextToken::from_js_ctx(ctx) }) else {
        return js_throw_type_error(ctx, "missing ctx user_data (call ridl_context_init)");
    };
    let scope = h.enter_scope();
    let mut env = mquickjs_rs::Env::new(&scope);
        let mut args: Vec<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>> = Vec::new();
    for i in 0..(argc as usize) {
    args.push(scope.value(unsafe { *argv.add(i) }));
    }



    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return js_throw_type_error(ctx, "missing ridl_ext (call ridl_context_init)");
    }
    let Some(slot_ptr) = (unsafe {
        ::mquickjs_rs::ridl_ext_access::ridl_get_erased_ctx_slot_by_name(
            ext_ptr,
            b"singleton_global_console".as_ptr(),
            b"singleton_global_console".len(),
        )
    }) else {
        return js_throw_type_error(ctx, "missing ridl ctx_ext vtable (call ridl_context_init)");
    };
    let slot = unsafe { &mut *slot_ptr };
    if !slot.is_set() {
        return js_throw_type_error(ctx, "singleton not initialized");
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::api::ConsoleSingleton>;
    // Synchronous method
    let singleton: &mut dyn crate::api::ConsoleSingleton = unsafe { &mut **holder_ptr };
    let result = singleton.log(
        &mut env, args
    );
    let _ = result;
let _ = ctx;
let _ = argc;
let _ = argv;
mquickjs_rs::mquickjs_ffi::JS_UNDEFINED
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_global_singleton_console_error(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue {
    let _ = this_val;
    // v_next (A2): per-JSContext singleton dispatch via ctx user_data -> ContextInner -> ridl_ext.
    let Some(h) = (unsafe { mquickjs_rs::context::ContextToken::from_js_ctx(ctx) }) else {
        return js_throw_type_error(ctx, "missing ctx user_data (call ridl_context_init)");
    };
    let scope = h.enter_scope();
    let mut env = mquickjs_rs::Env::new(&scope);
        let mut args: Vec<mquickjs_rs::handles::local::Local<'_, mquickjs_rs::handles::local::Value>> = Vec::new();
    for i in 0..(argc as usize) {
    args.push(scope.value(unsafe { *argv.add(i) }));
    }



    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return js_throw_type_error(ctx, "missing ridl_ext (call ridl_context_init)");
    }
    let Some(slot_ptr) = (unsafe {
        ::mquickjs_rs::ridl_ext_access::ridl_get_erased_ctx_slot_by_name(
            ext_ptr,
            b"singleton_global_console".as_ptr(),
            b"singleton_global_console".len(),
        )
    }) else {
        return js_throw_type_error(ctx, "missing ridl ctx_ext vtable (call ridl_context_init)");
    };
    let slot = unsafe { &mut *slot_ptr };
    if !slot.is_set() {
        return js_throw_type_error(ctx, "singleton not initialized");
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::api::ConsoleSingleton>;
    // Synchronous method
    let singleton: &mut dyn crate::api::ConsoleSingleton = unsafe { &mut **holder_ptr };
    let result = singleton.error(
        &mut env, args
    );
    let _ = result;
let _ = ctx;
let _ = argc;
let _ = argv;
mquickjs_rs::mquickjs_ffi::JS_UNDEFINED
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_global_singleton_console_get_enabled(ctx: *mut JSContext, this_val: *mut JSValue, argc: c_int, argv: *mut JSValue) -> JSValue {
    let _ = this_val;
    let _ = argc;
    let _ = argv;

    // v_next (A2): per-JSContext singleton dispatch via ctx user_data -> ContextInner -> ridl_ext.
    let Some(h) = (unsafe { mquickjs_rs::context::ContextToken::from_js_ctx(ctx) }) else {
        return js_throw_type_error(ctx, "missing ctx user_data (call ridl_context_init)");
    };

    let ext_ptr = h.inner.ridl_ext_ptr();
    if ext_ptr.is_null() {
        return js_throw_type_error(ctx, "missing ridl_ext (call ridl_context_init)");
    }
    let Some(slot_ptr) = (unsafe {
        ::mquickjs_rs::ridl_ext_access::ridl_get_erased_ctx_slot_by_name(
            ext_ptr,
            b"singleton_global_console".as_ptr(),
            b"singleton_global_console".len(),
        )
    }) else {
        return js_throw_type_error(ctx, "missing ridl ctx_ext vtable (call ridl_context_init)");
    };
    let slot = unsafe { &mut *slot_ptr };
    if !slot.is_set() {
        return js_throw_type_error(ctx, "singleton not initialized");
    }

    let holder_ptr = slot.ptr() as *mut Box<dyn crate::api::ConsoleSingleton>;
    let singleton: &mut dyn crate::api::ConsoleSingleton = unsafe { &mut **holder_ptr };

    let result = singleton.enabled();
    mquickjs_rs::mquickjs_ffi::js_mkbool(result)
}

// Interface implementations

// -----------------------------------------------------------------------------
// RIDL class glue (constructors/finalizers/methods/properties/proto properties)
// -----------------------------------------------------------------------------