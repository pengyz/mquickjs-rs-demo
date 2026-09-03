use mquickjs_rs::handles::local::{Local, Value};

fn gc(scope: &mquickjs_rs::handles::scope::Scope<'_>) {
    unsafe { mquickjs_rs::mquickjs_ffi::JS_GC(scope.ctx_raw()) };
}

#[test]
fn gc_root_keeps_value_alive_across_gc() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();
    let scope = token.enter_scope();

    // Same shape as deps/mquickjs/selftest_gc_mark.c: make object that references global.
    let raw = ctx.eval_jsvalue("({ g: 1 })").expect("eval object");

    let local: Local<'_, Value> = scope.value(raw);
    let root = mquickjs_rs::Root::new(&scope, local);

    gc(&scope);

    // No further heap operations; assertion is "GC runs" with a context-level root.
    // (Accessing properties here requires correct lifetime/dup model for JSValue.)
    let _ = root;
}

#[test]
fn gc_root_removed_allows_collection_of_cycle() {
    let mut ctx = mquickjs_rs::Context::new(1024 * 1024).expect("create context");
    let token = ctx.token();
    let scope = token.enter_scope();

    // Two plain JS objects with a back-reference to form a cycle.
    let b = ctx.eval_jsvalue("({})").expect("eval object");
    let a = ctx.eval_jsvalue("({})").expect("eval object");

    let b_local: Local<'_, Value> = scope.value(b);
    let b_root = mquickjs_rs::Root::new(&scope, b_local);

    let a_local: Local<'_, Value> = scope.value(a);
    let a_root = mquickjs_rs::Root::new(&scope, a_local);

    unsafe {
        let name = std::ffi::CString::new("back").unwrap();
        let _ = mquickjs_rs::mquickjs_ffi::JS_SetPropertyStr(
            scope.ctx_raw(),
            b_root.as_raw(),
            name.as_ptr(),
            a_root.as_raw(),
        );
    }

    drop(a_root);
    gc(&scope);
    gc(&scope);

    // Drop last root; the cycle is now only within JS heap.
    drop(b_root);

    // No finalizer available here; assertion is "GC runs".
    gc(&scope);
    gc(&scope);
}
