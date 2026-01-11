use core::ffi::c_void;

use crate::ridl_runtime::ErasedSingletonSlot;

/// A vtable describing how to access a singleton slot within an app-defined `CtxExt`.
///
/// The app build-time aggregator generates one `RidlCtxExtVTable` (and slot accessors)
/// matching the concrete `CtxExt` layout.
#[repr(C)]
pub struct RidlCtxExtVTable {
    pub get_slot: unsafe extern "C" fn(ext_ptr: *mut c_void, slot_index: u32) -> *mut ErasedSingletonSlot,
}

static mut RIDL_CTX_EXT_VTABLE: *const RidlCtxExtVTable = core::ptr::null();

/// Called by app-generated `ridl_context_init` once per process.
///
/// Safety: vtable must remain valid for the duration of the process.
pub unsafe fn ridl_set_ctx_ext_vtable(vt: &'static RidlCtxExtVTable) {
    RIDL_CTX_EXT_VTABLE = vt as *const _;
}

/// Get a mutable pointer to an erased singleton slot by index.
///
/// Safety: `ext_ptr` must point to the app's `CtxExt` instance (stored behind ContextInner.ridl_ext_ptr).
#[inline]
pub unsafe fn ridl_get_erased_singleton_slot(
    ext_ptr: *mut c_void,
    slot_index: u32,
) -> Option<*mut ErasedSingletonSlot> {
    let vt = RIDL_CTX_EXT_VTABLE;
    if vt.is_null() {
        return None;
    }
    let p = ((*vt).get_slot)(ext_ptr, slot_index);
    if p.is_null() {
        None
    } else {
        Some(p)
    }
}

/// Check whether the app has installed the ctx-ext vtable.
#[inline]
pub fn ridl_ctx_ext_vtable_is_set() -> bool {
    unsafe { !RIDL_CTX_EXT_VTABLE.is_null() }
}
