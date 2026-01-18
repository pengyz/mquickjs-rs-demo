use mquickjs_rs::{Context, HandleScope};

fn main() {
    let ctx = Context::new(1024 * 1024).unwrap();
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut outer = HandleScope::new(&scope);

    let scope_ref = outer.scope();

    // This must NOT compile:
    // a handle created inside an EscapableHandleScope cannot leave the closure
    // unless it is escaped.
    let _h_inner = outer.escapable(|mut inner| {
        let v = ctx.create_string(scope_ref, "x").unwrap();
        let h = inner.handle(v);
        // missing: inner.escape(...)
        h
    });
}
