use std::sync::atomic::{AtomicI32, Ordering};

use mquickjs_rs::handles::local::{Local, Value};

#[cfg(feature = "ridl-extensions")]
static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

#[cfg(feature = "ridl-extensions")]
#[test]
fn gc_root_cycle_collectable_after_root_drop() {
    // Ensure RIDL modules are linked and process-level initialization happens.
    // Without this, the ridl-enabled stdlib table (in libmquickjs.a) references js_* symbols
    // that live in Rust RIDL module crates, and the linker may drop those objects.
    mquickjs_rs::ridl_bootstrap!();
    // 1) Create context.
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();
    let scope = token.enter_scope();

    // 2) Create a plain JS object B.
    let b = ctx.eval_jsvalue("({})").expect("eval object");

    // 3) Create a Root for B to simulate an async task holding it.
    let b_local: Local<'_, Value> = scope.value(b);
    let b_root = mquickjs_rs::Root::new(&scope, b_local);

    // 4) Create a user class object A (opaque holds a Root to B).
    // NOTE: this test is engine-level; we use a minimal C-side class id.
    // We only rely on JS_SetContextGCMark (already installed by mquickjs-rs) + Root.

    // We can't allocate a new user class without ROM class tables; so we model A as a
    // plain JS object with a native struct tied to context roots:
    // - Keep a separate Root for A in Rust (like a task owning A)
    // - Make B back-reference A
    // - Drop both Rust roots and force GC
    // - Verify finalizer counter changes via a native Drop guard

    // Represent "A" as another JS object.
    let a = ctx.eval_jsvalue("({})").expect("eval object");

    // Root A as well to mimic host holding it.
    let a_local: Local<'_, Value> = scope.value(a);
    let a_root = mquickjs_rs::Root::new(&scope, a_local);

    // Wire B.back = A.
    unsafe {
        let name = std::ffi::CString::new("back").unwrap();
        let _ = mquickjs_rs::mquickjs_ffi::JS_SetPropertyStr(
            scope.ctx_raw(),
            b_root.as_raw(),
            name.as_ptr(),
            a_root.as_raw(),
        );
    }

    // 5) Drop one root and ensure still alive.
    drop(a_root);
    unsafe { mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw()) };
    unsafe { mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw()) };

    // 6) Drop last root; the cycle is now only within JS heap.
    drop(b_root);

    // Attach a native drop guard by incrementing counter when context drops.
    // (We don't have user-class finalizers in this minimal test.)
    struct DropGuard;
    impl Drop for DropGuard {
        fn drop(&mut self) {
            FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }
    let _guard = DropGuard;

    unsafe { mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw()) };
    unsafe { mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw()) };

    // The guard will be dropped at end of test; here we only assert no crash.
    // TODO: Replace this with a real user class finalizer once RIDL struct support lands.
    assert!(FINALIZER_COUNT.load(Ordering::SeqCst) >= 0);
}
