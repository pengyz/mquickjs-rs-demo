use crate::mquickjs_ffi;

/// A GC-traced field for use inside RIDL user class opaque structs.
///
/// `Traced<T>` holds a raw `JSValue` without registering in the `RootsRegistry`.
/// Unlike `Root<T>`, it does NOT create an independent GC root. Instead, it
/// relies on the owner object's `gc_mark` callback to mark this value during
/// the GC mark phase.
///
/// When the owner object is reachable, its `gc_mark` marks all `Traced<T>`
/// fields, keeping the referenced JS objects alive. When the owner becomes
/// unreachable, the `Traced<T>` fields are also unreachable and will be
/// collected.
///
/// # Safety Model
///
/// **Invariant**: `Traced<T>` lifetime ≤ JS object lifetime
///
/// This is guaranteed by:
/// 1. JS object reachable → opaque valid → `Traced<T>` valid
/// 2. JS object unreachable → GC triggers finalizer → opaque Drop → `Traced<T>` invalidated
///
/// **Must be used within RIDL user class opaque only.**
///
/// ## Violation Scenarios (MUST avoid)
///
/// ```rust,no_run
/// // ❌ Taking Traced out of opaque
/// // let traced = opaque.held.take(); // opaque may be finalized later
/// // traced now holds dangling pointer
///
/// // ❌ Cross-Context usage
/// // let traced_from_ctx1 = /* ... */;
/// // Use in ctx2 → undefined behavior
/// ```
///
/// ## Design Decision
///
/// - No runtime checks (performance consideration)
/// - Relies on type system + RIDL constraints for safety
/// - No Drop implementation (does not call JS_FreeValue/JS_DupValue)
pub struct Traced<T = crate::handles::local::Value> {
    raw: mquickjs_ffi::JSValue,
    _t: std::marker::PhantomData<T>,
}

impl<T> Traced<T> {
    /// Create a `Traced<T>` from a `Local<T>`.
    ///
    /// # Safety
    ///
    /// The caller must ensure this `Traced` is stored in a RIDL user class opaque
    /// that will be marked by the class's `gc_mark` callback.
    pub fn new(v: crate::handles::local::Local<'_, T>) -> Self {
        Self {
            raw: v.as_raw(),
            _t: std::marker::PhantomData,
        }
    }

    /// Get the raw JSValue.
    pub fn as_raw(&self) -> mquickjs_ffi::JSValue {
        self.raw
    }

    /// Check if this Traced holds a value (not null/undefined).
    pub fn is_some(&self) -> bool {
        // JSValue is a u64; null/undefined are special tag values, not pointer null.
        // A zero JSValue is not a valid value.
        self.raw != 0
    }

    /// Mark this traced value during GC.
    ///
    /// Called by the auto-generated gc_mark callback in RIDL user class opaque structs.
    /// This marks the referenced JSValue as reachable, preventing it from being
    /// collected while the owner object is alive.
    ///
    /// # Safety
    ///
    /// `mf` must be a valid mark function pointer provided by the engine during GC.
    pub unsafe fn gc_mark(&self, mf: *const mquickjs_ffi::JSMarkFunc) {
        if let Some(mark_value) = unsafe { (*mf).mark_value } {
            unsafe { mark_value(mf, self.raw) };
        }
    }
}

// No Drop implementation: Traced does not manage lifetime independently.
// The referenced JSValue is kept alive by the owner's gc_mark callback.
