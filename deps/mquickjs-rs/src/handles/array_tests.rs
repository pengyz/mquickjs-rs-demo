use crate::Context;
use crate::Env;

#[test]
fn array_set_out_of_bounds_is_error() {
    let ctx = Context::new(1024 * 1024).unwrap();
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut env = Env::new(&scope);

    let arr = env.array().unwrap();
    let arr_local = scope.value(arr.as_raw()).try_into_array(&scope).unwrap();

    // push one element
    let one = env.int_local(1).unwrap();
    let _ = arr_local.push(&env, one).unwrap();

    // index > len must throw TypeError in mquickjs (no holes)
    let len = arr_local.len(&env).unwrap();
    assert_eq!(len, 1);

    let two = env.int_local(2).unwrap();
    let r = arr_local.set(&env, len + 9, two);
    assert!(r.is_err());
}

#[test]
fn array_set_at_end_extends() {
    let ctx = Context::new(1024 * 1024).unwrap();
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut env = Env::new(&scope);

    let arr = env.array().unwrap();
    let arr_local = scope.value(arr.as_raw()).try_into_array(&scope).unwrap();

    let len0 = arr_local.len(&env).unwrap();
    assert_eq!(len0, 0);

    let x = env.str("x").unwrap();
    let x_raw = x.as_raw();
    arr_local.set(&env, 0, scope.value(x_raw)).unwrap();

    let len1 = arr_local.len(&env).unwrap();
    assert_eq!(len1, 1);

    let v0 = arr_local.get(&mut env, 0).unwrap();
    let v0_raw = v0.as_raw();
    let s = env.get_string(scope.value(v0_raw)).unwrap();
    assert_eq!(s, "x");
}

#[test]
fn array_pop_shrinks() {
    let ctx = Context::new(1024 * 1024).unwrap();
    let token = ctx.token();
    let scope = token.enter_scope();

    let mut env = Env::new(&scope);

    let arr = env.array().unwrap();
    let arr_local = scope.value(arr.as_raw()).try_into_array(&scope).unwrap();

    let a = env.str("a").unwrap();
    let a_raw = a.as_raw();
    arr_local.push(&env, scope.value(a_raw)).unwrap();

    let b = env.str("b").unwrap();
    let b_raw = b.as_raw();
    arr_local.push(&env, scope.value(b_raw)).unwrap();

    let v = arr_local.pop(&mut env).unwrap();
    let v_raw = v.as_raw();
    let s = env.get_string(scope.value(v_raw)).unwrap();
    assert_eq!(s, "b");

    let len = arr_local.len(&env).unwrap();
    assert_eq!(len, 1);
}
