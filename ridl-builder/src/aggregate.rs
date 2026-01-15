use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Module {
    pub crate_name: String,
    pub crate_dir: PathBuf,
    pub ridl_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AggregateOutput {
    pub manifest_path: PathBuf,
    pub ridl_register_h: PathBuf,
}

pub fn default_out_dir(target_dir: &Path, app_id: &str) -> PathBuf {
    target_dir
        .join("ridl")
        .join("apps")
        .join(app_id)
        .join("aggregate")
}

pub fn write_manifest(out_dir: &Path, modules: &[Module]) -> std::io::Result<PathBuf> {
    #[derive(serde::Serialize)]
    struct Manifest<'a> {
        schema_version: u32,
        modules: Vec<ManifestModule<'a>>,
    }

    #[derive(serde::Serialize)]
    struct ManifestModule<'a> {
        crate_name: &'a str,
        crate_dir: String,
        ridl_files: Vec<String>,
    }

    let modules = modules
        .iter()
        .map(|m| ManifestModule {
            crate_name: &m.crate_name,
            crate_dir: m.crate_dir.display().to_string(),
            ridl_files: m.ridl_files.iter().map(|f| f.display().to_string()).collect(),
        })
        .collect();

    let manifest = Manifest {
        schema_version: 1,
        modules,
    };

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    fs::create_dir_all(out_dir)?;
    let path = out_dir.join("ridl-manifest.json");
    fs::write(&path, json)?;
    Ok(path)
}

pub fn write_ridl_shared_files_and_context_init(
    out_dir: &Path,
    modules: &[Module],
) -> std::io::Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    fs::create_dir_all(out_dir)?;

    let mut ridl_files: Vec<String> = Vec::new();
    for m in modules {
        for f in &m.ridl_files {
            ridl_files.push(f.display().to_string());
        }
    }


    let plan = ridl_tool::plan::RidlPlan {
        schema_version: 1,
        cargo_toml: PathBuf::new(),
        modules: modules
            .iter()
            .map(|m| ridl_tool::plan::RidlModule {
                crate_name: m.crate_name.clone(),
                name: m.crate_name.clone(),
                crate_dir: m.crate_dir.clone(),
                ridl_files: m.ridl_files.clone(),
            })
            .collect(),
        generated: ridl_tool::plan::GeneratedPaths {
            out_dir: out_dir.to_path_buf(),
            mquickjs_ridl_register_h: out_dir.join("mquickjs_ridl_register.h"),
            mqjs_ridl_user_class_ids_h: out_dir.join("mqjs_ridl_user_class_ids.h"),
            ridl_class_id_rs: out_dir.join("ridl_class_id.rs"),
        },
        inputs: Vec::new(),
    };

    // Consolidated aggregate outputs (ridl_symbols.rs + ridl_runtime_support.rs + ridl_bootstrap.rs)
    ridl_tool::generator::generate_aggregate_consolidated(&plan, out_dir)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok((
        out_dir.join("mquickjs_ridl_register.h"),
        out_dir.join("ridl_symbols.rs"),
        out_dir.join("ridl_runtime_support.rs"),
        out_dir.join("ridl_bootstrap.rs"),
    ))
}


pub fn aggregate(target_dir: &Path, app_id: &str, modules: &[Module]) -> std::io::Result<AggregateOutput> {
    let out_dir = default_out_dir(target_dir, app_id);

    let manifest_path = write_manifest(&out_dir, modules)?;

    let (ridl_register_h, _ridl_symbols_rs, _ridl_runtime_support_rs, _ridl_bootstrap_rs) =
        write_ridl_shared_files_and_context_init(&out_dir, modules)?;

    Ok(AggregateOutput {
        manifest_path,
        ridl_register_h,
    })
}

