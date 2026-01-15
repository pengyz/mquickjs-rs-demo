use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::plan::{GeneratedPaths, RidlModule, RidlPlan};

#[derive(Debug, Deserialize)]
struct CargoToml {
    dependencies: Option<std::collections::BTreeMap<String, Dependency>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Dependency {
    #[allow(dead_code)]
    Simple(String),
    Detailed(DependencyDetail),
}

#[derive(Debug, Deserialize)]
struct DependencyDetail {
    path: Option<String>,
    #[allow(dead_code)]
    package: Option<String>,
}

pub fn resolve_from_cargo_toml(cargo_toml_path: &Path, out_dir: &Path) -> Result<RidlPlan, String> {
    let cargo_toml_path = cargo_toml_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize {}: {e}", cargo_toml_path.display()))?;

    let cargo_dir = cargo_toml_path
        .parent()
        .ok_or_else(|| "Cargo.toml has no parent directory".to_string())?;

    let text = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|e| format!("Failed to read {}: {e}", cargo_toml_path.display()))?;
    let parsed: CargoToml = toml::from_str(&text)
        .map_err(|e| format!("Failed to parse {}: {e}", cargo_toml_path.display()))?;

    let mut modules: Vec<RidlModule> = Vec::new();
    let mut inputs: BTreeSet<PathBuf> = BTreeSet::new();
    inputs.insert(cargo_toml_path.clone());

    let Some(deps) = parsed.dependencies else {
        return Ok(empty_plan(&cargo_toml_path, out_dir, inputs));
    };

    for (dep_name, dep) in deps {
        let dep_path = match dep {
            Dependency::Simple(_) => continue,
            Dependency::Detailed(d) => d.path,
        };

        let Some(dep_path) = dep_path else { continue };
        let crate_dir = cargo_dir.join(dep_path);
        let crate_dir = match crate_dir.canonicalize() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let src_dir = crate_dir.join("src");
        let ridl_files = find_ridl_files(&src_dir);
        if ridl_files.is_empty() {
            continue;
        }

        for f in &ridl_files {
            inputs.insert(f.clone());
        }

        modules.push(RidlModule {
            crate_name: dep_name.clone(),
            name: dep_name,
            crate_dir,
            ridl_files,
        });
    }

    modules.sort_by(|a, b| a.name.cmp(&b.name));

    let generated = GeneratedPaths {
        out_dir: out_dir.to_path_buf(),
        mquickjs_ridl_register_h: out_dir.join("mquickjs_ridl_register.h"),
        mqjs_ridl_user_class_ids_h: out_dir.join("mqjs_ridl_user_class_ids.h"),
        ridl_class_id_rs: out_dir.join("ridl_class_id.rs"),
    };

    Ok(RidlPlan {
        schema_version: 1,
        cargo_toml: cargo_toml_path,
        modules,
        generated,
        inputs: inputs.into_iter().collect(),
    })
}

fn empty_plan(cargo_toml: &Path, out_dir: &Path, inputs: BTreeSet<PathBuf>) -> RidlPlan {
    let generated = GeneratedPaths {
        out_dir: out_dir.to_path_buf(),
        mquickjs_ridl_register_h: out_dir.join("mquickjs_ridl_register.h"),
        mqjs_ridl_user_class_ids_h: out_dir.join("mqjs_ridl_user_class_ids.h"),
        ridl_class_id_rs: out_dir.join("ridl_class_id.rs"),
    };
    RidlPlan {
        schema_version: 1,
        cargo_toml: cargo_toml.to_path_buf(),
        modules: vec![],
        generated,
        inputs: inputs.into_iter().collect(),
    }
}

fn find_ridl_files(src_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !src_dir.is_dir() {
        return out;
    }

    let Ok(rd) = std::fs::read_dir(src_dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("ridl") {
            if let Ok(cp) = p.canonicalize() {
                out.push(cp);
            }
        }
    }

    out.sort();
    out
}
