// Intentionally minimal: we only care that generated identifiers compile.

use mquickjs_rs::Env;

pub struct DefaultRustIdentSingleton;

impl crate::api::RustIdentSingleton for DefaultRustIdentSingleton {
    fn type_(&mut self, _type_: i32, _match_: i32) -> i32 {
        0
    }

    fn ref_(&mut self, _ref_: i32) -> i32 {
        0
    }

    fn self_(&mut self, _self_: i32) -> i32 {
        0
    }

    fn crate_(&mut self, _crate_: i32) -> i32 {
        0
    }

    fn super_(&mut self, _super_: i32) -> i32 {
        0
    }

    fn dyn_(&mut self, _dyn_: i32) -> i32 {
        0
    }

    fn foo_bar(&mut self, _foo_bar: i32, _1st: i32) -> i32 {
        let _ = _1st;
        0
    }
}

pub fn create_rust_ident_singleton() -> Box<dyn crate::api::RustIdentSingleton> {
    Box::new(DefaultRustIdentSingleton)
}

// The generator may also emit direct Rust-callable helpers for functions; keep an Env import
// available to avoid unused-import churn when templates evolve.
#[allow(dead_code)]
fn _keep_env(_env: &mut Env<'_>) {}
