#[test]
fn compile_fail() {
    // trybuild 会在 target/tests/trybuild/ 下发起**嵌套的 cargo 构建**来编译用例。
    // 那些 crate 不属于本包，因此拿不到本包 build.rs 通过 `rustc-link-arg`
    // 指定的 native stdlib —— 而任何链接 mquickjs-rs 的二进制都需要
    // `js_stdlib`（定义在变体专属的 mqjs_stdlib_impl.o 中）。
    //
    // 这里把 base 变体 stdlib 的链接参数注入 RUSTFLAGS，供嵌套构建使用。
    // 选 base：其 js_c_function_table 不引用任何 RIDL 模块符号。
    if let Some(args) = mquickjs_rs::native_test_link_args() {
        let prev = std::env::var("RUSTFLAGS").unwrap_or_default();
        // Safety: 本测试不与其他测试并发修改该环境变量；
        // 设置后立即用于随后的嵌套构建。
        unsafe { std::env::set_var("RUSTFLAGS", format!("{prev} {args}")) };
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.fail.rs");
    t.pass("tests/compile_fail/*.pass.rs");
}