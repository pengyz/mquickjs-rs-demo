use mquickjs_rs::{Context, HandleScope};

fn main() {
    let ctx = Context::new(1024 * 1024).unwrap();
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut outer = HandleScope::new(&scope);

    let scope_ref = outer.scope();

    let escaped = outer.escapable(|mut inner| {
        let v = ctx.create_string(scope_ref, "x").unwrap();
        let h = inner.handle(v);
        inner.escape(h)
    });

    let _ = escaped;
}
