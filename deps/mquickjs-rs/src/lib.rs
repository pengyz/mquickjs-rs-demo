// 导入生成的绑定
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(dead_code)]
#[allow(clippy::all)]
mod mquickjs_ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// 导入各个模块
pub mod context;
pub mod value;
pub mod object;
pub mod function;

// 重新导出模块中的公共项
pub use context::Context;
pub use value::Value;
pub use object::Object;
pub use function::Function;
