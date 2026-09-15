fn main() {
    // Delegates all RIDL OUT_DIR wiring to the shared build-helper crate.
    mquickjs_ridl_glue::emit();

    // 本 crate 是「拥有完整 RIDL 模块集合的叶子」，因此由它链接 ridl 变体的
    // C stdlib（其 js_c_function_table 引用本应用全部 RIDL 模块的 C 入口）。
    // mquickjs-rs 自身的 test 目标不受影响 —— 那里固定链接 base stdlib。
    mquickjs_ridl_glue::emit_native_stdlib_link();
}