use core::ffi::c_void;

use crate::ridl_ext_access;

/// A type-erased ctx slot stored in per-context extension state.
///
/// This is a RIDL runtime primitive: module-generated code can store a pointer to a module-defined
/// state object (singleton state, class prototype state, etc), plus a drop function to release it
/// at JSContext teardown.
pub struct ErasedCtxSlot {
    ptr: *mut c_void,
    drop_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

/// Per-slot erased vtable exported by modules.
///
/// The app-side aggregator uses this to allocate a module-owned state object and register it into
/// a ctx-ext slot without naming the module's Rust types/traits.
#[repr(C)]
pub struct RidlErasedSlotVTable {
    /// Create the allocation. Must return an opaque pointer owned by the module.
    pub create: unsafe extern "C" fn() -> *mut c_void,
    /// Drop the allocation previously returned by `create`.
    pub drop: unsafe extern "C" fn(*mut c_void),
}

impl ErasedCtxSlot {
    pub const fn empty() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            drop_fn: None,
        }
    }

    pub fn is_set(&self) -> bool {
        !self.ptr.is_null()
    }

    pub fn ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Safety: caller must ensure (ptr, drop_fn) match the allocation.
    pub unsafe fn set(&mut self, ptr: *mut c_void, drop_fn: unsafe extern "C" fn(*mut c_void)) {
        debug_assert!(self.ptr.is_null());
        self.ptr = ptr;
        self.drop_fn = Some(drop_fn);
    }

    /// Safety: may only be called once.
    pub unsafe fn drop_in_place(&mut self) {
        if let Some(f) = self.drop_fn.take() {
            let p = core::mem::replace(&mut self.ptr, core::ptr::null_mut());
            if !p.is_null() {
                f(p);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RidlSlotSetError {
    /// The app did not install the ctx-ext vtable (or it was cleared), so slot lookup is impossible.
    VTableMissing,
    /// slot_index is not a valid slot for the current app-generated CtxExt layout.
    InvalidSlot { slot_index: u32 },
    /// The slot was already set and overwriting is not allowed.
    AlreadySet { slot_index: u32 },
}

pub trait RidlSlotWriter {
    /// Set an erased singleton slot.
    ///
    /// Safety: (ptr, drop_fn) must match the allocation and drop strategy.
    unsafe fn set_slot(
        &mut self,
        slot_index: u32,
        ptr: *mut c_void,
        drop_fn: unsafe extern "C" fn(*mut c_void),
    ) -> Result<(), RidlSlotSetError>;
}

pub struct RidlCtxExtWriter {
    ext_ptr: *mut c_void,
}

impl RidlCtxExtWriter {
    /// Safety: `ext_ptr` must point to the app-owned `CtxExt` stored in `ContextInner`.
    pub unsafe fn new(ext_ptr: *mut c_void) -> Self {
        Self { ext_ptr }
    }
}

impl RidlSlotWriter for RidlCtxExtWriter {
    unsafe fn set_slot(
        &mut self,
        slot_index: u32,
        ptr: *mut c_void,
        drop_fn: unsafe extern "C" fn(*mut c_void),
    ) -> Result<(), RidlSlotSetError> {
        let Some(slot_ptr) =
            ridl_ext_access::ridl_get_erased_ctx_slot(self.ext_ptr, slot_index)
        else {
            return Err(RidlSlotSetError::VTableMissing);
        };

        let slot = unsafe { &mut *slot_ptr };
        if slot.is_set() {
            return Err(RidlSlotSetError::AlreadySet { slot_index });
        }

        // SAFETY: caller provides allocation + drop function pairing.
        unsafe { slot.set(ptr, drop_fn) };
        Ok(())
    }
}
