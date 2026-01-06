mod stdlib_demo_impl;
mod stdlib_demo_glue;

// 导出胶水代码模块，使外部可以使用
pub use stdlib_demo_glue::*;