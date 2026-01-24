use crate::handles::handle_scope::HandleScope;
use crate::Context;

#[test]
fn escapable_escape_survives_gc() {
    let context = Context::new(1024 * 1024).unwrap();

    let token = context.token();
    let scope = token.enter_scope();

    let mut outer = HandleScope::new(&scope);
    let scope_ref = outer.scope();

    let escaped = outer.escapable(|mut inner| {
        let v = context.create_string(scope_ref, "escaped").unwrap();
        let h = inner.handle(v);
        inner.escape(h)
    });

    unsafe { crate::mquickjs_ffi::JS_GC(context.ctx) };

    let s = context
        .get_string(scope_ref.value(escaped.as_raw()))
        .unwrap();
    assert_eq!(s, "escaped");
}

#[test]
fn handle_scope_pins_value_survives_gc() {
    let context = Context::new(1024 * 1024).unwrap();

    let token = context.token();
    let scope = token.enter_scope();

    let mut hs = HandleScope::new(&scope);

    let v = context.create_string(&scope, "pinned").unwrap();
    let h = hs.handle(v);

    unsafe { crate::mquickjs_ffi::JS_GC(context.ctx) };

    let s = context.get_string(scope.value(h.as_raw())).unwrap();
    assert_eq!(s, "pinned");
}
