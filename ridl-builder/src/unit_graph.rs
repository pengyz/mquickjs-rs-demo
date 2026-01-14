use std::{
    collections::HashSet,
    process::Command,
};

use serde::Deserialize;

use crate::{CargoSubcommand, CargoMetadata, CargoPackage};

pub fn direct_deps_from_unit_graph<'a>(
    cargo_toml: &std::path::Path,
    meta: &'a CargoMetadata,
    app_pkg: &'a CargoPackage,
    subcommand: CargoSubcommand,
    cargo_args: &[String],
) -> Vec<&'a CargoPackage> {
    let raw = run_unit_graph(cargo_toml, subcommand, cargo_args);
    direct_deps_from_unit_graph_raw(meta, app_pkg, subcommand, &raw)
}

pub fn direct_deps_auto_detect<'a>(
    cargo_toml: &std::path::Path,
    meta: &'a CargoMetadata,
    app_pkg: &'a CargoPackage,
    cargo_args: &[String],
) -> Result<Vec<&'a CargoPackage>, String> {
    // Prefer build-mode graph for module discovery in default path.
    // If users want dev-deps, they can pass `--intent test` explicitly.
    let sc = CargoSubcommand::Build;
    let raw = try_run_unit_graph(cargo_toml, sc, cargo_args)?;
    Ok(direct_deps_from_unit_graph_raw(meta, app_pkg, sc, &raw))
}

pub fn direct_deps_from_unit_graph_raw<'a>(
    meta: &'a CargoMetadata,
    app_pkg: &'a CargoPackage,
    subcommand: CargoSubcommand,
    raw: &[u8],
) -> Vec<&'a CargoPackage> {
    let ug: UnitGraph = serde_json::from_slice(raw).unwrap_or_else(|e| {
        panic!(
            "failed to parse cargo unit-graph json ({} bytes): {e}",
            raw.len()
        )
    });

    let app_pkg_id = &app_pkg.id;

    let entry_unit_indices: Vec<usize> = ug
        .units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.pkg_id == *app_pkg_id)
        .filter(|(_, u)| is_entry_unit(subcommand, u))
        .map(|(idx, _)| idx)
        .collect();

    if entry_unit_indices.is_empty() {
        panic!(
            "unit-graph: no entry units found for app pkg id={}",
            app_pkg_id
        );
    }

    // Collect 1-hop deps from entry units.
    let mut dep_pkg_ids: HashSet<&str> = HashSet::new();
    for idx in entry_unit_indices {
        let u = &ug.units[idx];
        for dep in &u.dependencies {
            let dep_idx = dep.index;
            let Some(dep_u) = ug.units.get(dep_idx) else {
                continue;
            };
            dep_pkg_ids.insert(dep_u.pkg_id.as_str());
        }
    }

    let mut out = Vec::new();
    for pkg_id in dep_pkg_ids {
        if pkg_id == app_pkg.id {
            continue;
        }

        let Some(pkg) = meta.packages.iter().find(|p| p.id == pkg_id) else {
            continue;
        };

        // Only accept dependencies with local sources (path/git checkout). For registry crates,
        // scanning their src/ for *.ridl is not part of our module story.
        if !pkg.manifest_path.exists() {
            continue;
        }

        out.push(pkg);
    }

    out
}

pub fn run_unit_graph(cargo_toml: &std::path::Path, subcommand: CargoSubcommand, cargo_args: &[String]) -> Vec<u8> {
    try_run_unit_graph(cargo_toml, subcommand, cargo_args).unwrap_or_else(|e| panic!("{e}"))
}

pub fn try_run_unit_graph(
    cargo_toml: &std::path::Path,
    subcommand: CargoSubcommand,
    cargo_args: &[String],
) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("cargo");
    match subcommand {
        CargoSubcommand::Build => {
            cmd.arg("build");
        }
        CargoSubcommand::Test => {
            cmd.arg("test").arg("--no-run");
        }
    }

    cmd.arg("-Z")
        .arg("unstable-options")
        .arg("--unit-graph")
        .arg("--manifest-path")
        .arg(cargo_toml)
        .args(cargo_args);

    let out = cmd.output().map_err(|e| format!("failed to run cargo --unit-graph: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo --unit-graph failed (exit={:?}). Hint: this requires nightly cargo (try `cargo +nightly ... -Z unstable-options --unit-graph`). stderr:\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    Ok(out.stdout)
}

fn is_entry_unit(subcommand: CargoSubcommand, u: &Unit) -> bool {
    let kinds: Vec<&str> = u.target.kind.iter().map(|s| s.as_str()).collect();

    match subcommand {
        CargoSubcommand::Build => kinds.iter().any(|k| matches!(*k, "lib" | "bin")),
        CargoSubcommand::Test => kinds.iter().any(|k| *k == "test"),
    }
}

#[derive(Deserialize)]
struct UnitGraph {
    units: Vec<Unit>,
}

#[derive(Deserialize)]
struct Unit {
    #[serde(rename = "pkg_id")]
    pkg_id: String,
    target: UnitTarget,
    #[serde(rename = "dependencies")]
    dependencies: Vec<UnitDep>,
}

#[derive(Deserialize)]
struct UnitTarget {
    kind: Vec<String>,
}

#[derive(Deserialize)]
struct UnitDep {
    index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CargoResolve;

    #[test]
    fn entry_unit_selection_build_accepts_lib_and_bin() {
        let lib = Unit {
            pkg_id: "root".to_string(),
            target: UnitTarget {
                kind: vec!["lib".to_string()],
            },
            dependencies: vec![],
        };
        assert!(is_entry_unit(CargoSubcommand::Build, &lib));

        let bin = Unit {
            pkg_id: "root".to_string(),
            target: UnitTarget {
                kind: vec!["bin".to_string()],
            },
            dependencies: vec![],
        };
        assert!(is_entry_unit(CargoSubcommand::Build, &bin));

        let build_script = Unit {
            pkg_id: "root".to_string(),
            target: UnitTarget {
                kind: vec!["custom-build".to_string()],
            },
            dependencies: vec![],
        };
        assert!(!is_entry_unit(CargoSubcommand::Build, &build_script));
    }

    #[test]
    fn entry_unit_selection_test_accepts_test() {
        let test = Unit {
            pkg_id: "root".to_string(),
            target: UnitTarget {
                kind: vec!["test".to_string()],
            },
            dependencies: vec![],
        };
        assert!(is_entry_unit(CargoSubcommand::Test, &test));

        let lib = Unit {
            pkg_id: "root".to_string(),
            target: UnitTarget {
                kind: vec!["lib".to_string()],
            },
            dependencies: vec![],
        };
        assert!(!is_entry_unit(CargoSubcommand::Test, &lib));
    }

    #[test]
    fn direct_deps_from_unit_graph_raw_collects_one_hop_deps_from_entry_units() {
        let mut meta = CargoMetadata {
            packages: vec![
                CargoPackage {
                    id: "root".to_string(),
                    name: "root".to_string(),
                    manifest_path: "/workspace/root/Cargo.toml".into(),
                },
                CargoPackage {
                    id: "a".to_string(),
                    name: "a".to_string(),
                    manifest_path: "/workspace/a/Cargo.toml".into(),
                },
                CargoPackage {
                    id: "b".to_string(),
                    name: "b".to_string(),
                    manifest_path: "/workspace/b/Cargo.toml".into(),
                },
                CargoPackage {
                    id: "transitive".to_string(),
                    name: "transitive".to_string(),
                    manifest_path: "/workspace/transitive/Cargo.toml".into(),
                },
            ],
            // Not used by unit-graph path.
            resolve: CargoResolve { nodes: vec![] },
            target_directory: "/workspace/target".into(),
        };

        // Ensure manifest_path.exists() passes for our fake local packages.
        // Use a writable temp dir (tests might not have permission to write to /workspace).
        let tmp = std::env::temp_dir().join("ridl-builder-tests").join("unit-graph-1");
        std::fs::create_dir_all(&tmp).unwrap();

        for p in &mut meta.packages {
            let rel = p
                .manifest_path
                .strip_prefix("/")
                .unwrap_or(&p.manifest_path);
            let dst = tmp.join(rel);
            if let Some(dir) = dst.parent() {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(&dst, "[package]\nname='x'\nversion='0.0.0'\n").unwrap();
            p.manifest_path = dst;
        }

        let app_pkg = &meta.packages[0];

        // Graph:
        //   unit0(root lib) -> unit1(a lib), unit2(b lib)
        //   unit1(a lib)    -> unit3(transitive lib)
        // direct deps should be {a,b}, not include transitive.
        let raw = br#"{
  "units": [
    {
      "pkg_id": "root",
      "target": {"kind": ["lib"]},
      "dependencies": [{"index": 1}, {"index": 2}]
    },
    {
      "pkg_id": "a",
      "target": {"kind": ["lib"]},
      "dependencies": [{"index": 3}]
    },
    {
      "pkg_id": "b",
      "target": {"kind": ["lib"]},
      "dependencies": []
    },
    {
      "pkg_id": "transitive",
      "target": {"kind": ["lib"]},
      "dependencies": []
    }
  ]
}"#;

        let direct = direct_deps_from_unit_graph_raw(&meta, app_pkg, CargoSubcommand::Build, raw);
        let mut names: Vec<&str> = direct.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn direct_deps_from_unit_graph_raw_test_mode_uses_test_entry_unit() {
        let mut meta = CargoMetadata {
            packages: vec![
                CargoPackage {
                    id: "root".to_string(),
                    name: "root".to_string(),
                    manifest_path: "/workspace2/root/Cargo.toml".into(),
                },
                CargoPackage {
                    id: "normal".to_string(),
                    name: "normal".to_string(),
                    manifest_path: "/workspace2/normal/Cargo.toml".into(),
                },
                CargoPackage {
                    id: "dev".to_string(),
                    name: "dev".to_string(),
                    manifest_path: "/workspace2/dev/Cargo.toml".into(),
                },
            ],
            resolve: CargoResolve { nodes: vec![] },
            target_directory: "/workspace2/target".into(),
        };

        let tmp = std::env::temp_dir().join("ridl-builder-tests").join("unit-graph-2");
        std::fs::create_dir_all(&tmp).unwrap();

        for p in &mut meta.packages {
            let rel = p
                .manifest_path
                .strip_prefix("/")
                .unwrap_or(&p.manifest_path);
            let dst = tmp.join(rel);
            if let Some(dir) = dst.parent() {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(&dst, "[package]\nname='x'\nversion='0.0.0'\n").unwrap();
            p.manifest_path = dst;
        }

        let app_pkg = &meta.packages[0];

        // root has both lib and test units:
        //   root lib -> normal
        //   root test -> normal, dev
        // For subcommand=test we expect deps from test unit (includes dev).
        let raw = br#"{
  "units": [
    {
      "pkg_id": "root",
      "target": {"kind": ["lib"]},
      "dependencies": [{"index": 1}]
    },
    {
      "pkg_id": "normal",
      "target": {"kind": ["lib"]},
      "dependencies": []
    },
    {
      "pkg_id": "root",
      "target": {"kind": ["test"]},
      "dependencies": [{"index": 1}, {"index": 3}]
    },
    {
      "pkg_id": "dev",
      "target": {"kind": ["lib"]},
      "dependencies": []
    }
  ]
}"#;

        let direct = direct_deps_from_unit_graph_raw(&meta, app_pkg, CargoSubcommand::Test, raw);
        let mut names: Vec<&str> = direct.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["dev", "normal"]);
    }
}
