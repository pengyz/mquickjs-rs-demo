use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct RidlPlan {
    pub schema_version: u32,
    pub cargo_toml: PathBuf,
    pub modules: Vec<RidlModule>,
    pub generated: GeneratedPaths,
    pub inputs: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RidlModule {
    /// Dependency key / Rust crate ident used by downstream Rust code (e.g. `stdlib_demo`).
    pub crate_name: String,
    /// Logical module name (currently equals dependency key).
    pub name: String,
    pub crate_dir: PathBuf,
    pub ridl_files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedPaths {
    pub out_dir: PathBuf,
    pub mquickjs_ridl_register_h: PathBuf,
}
