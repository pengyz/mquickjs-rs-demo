#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.fail.rs");
    t.pass("tests/compile_fail/*.pass.rs");
}
