use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{AggregateOpts, CargoMetadata, Intent, aggregate::Module};

const DEFAULT_FALLBACK_INTENT: Intent = Intent::Build;

pub fn discover_ridl_modules(opts: &AggregateOpts) -> Vec<Module> {
    let meta = crate::cargo_metadata(&opts.cargo_toml);
    let app_pkg = crate::select_app_package(&meta, &opts.cargo_toml);

    let direct = if let Some(sc) = opts.cargo_subcommand {
        crate::unit_graph::direct_deps_from_unit_graph(
            &opts.cargo_toml,
            &meta,
            app_pkg,
            sc,
            &opts.cargo_args,
        )
    } else if opts.intent.is_some() {
        direct_deps_from_metadata(
            &meta,
            app_pkg,
            opts.intent.unwrap_or(DEFAULT_FALLBACK_INTENT),
        )
    } else {
        // Default path: try unit-graph without requiring extra flags.
        // If unit-graph isn't available (e.g. stable toolchain), fallback to metadata with default intent.
        match crate::unit_graph::direct_deps_auto_detect(
            &opts.cargo_toml,
            &meta,
            app_pkg,
            &opts.cargo_args,
        ) {
            Ok(direct) => direct,
            Err(_) => direct_deps_from_metadata(&meta, app_pkg, DEFAULT_FALLBACK_INTENT),
        }
    };

    let mut modules = Vec::new();
    for pkg in direct {
        let crate_dir = pkg
            .manifest_path
            .parent()
            .unwrap_or_else(|| {
                panic!(
                    "package.manifest_path has no parent: {}",
                    pkg.manifest_path.display()
                )
            })
            .to_path_buf();

        let ridl_files = find_ridl_files(&crate_dir.join("src"));
        if ridl_files.is_empty() {
            continue;
        }

        modules.push(Module {
            crate_name: pkg.name.clone(),
            crate_dir,
            ridl_files,
        });
    }

    modules
}

fn direct_deps_from_metadata<'a>(
    meta: &'a CargoMetadata,
    app_pkg: &'a crate::CargoPackage,
    intent: Intent,
) -> Vec<&'a crate::CargoPackage> {
    let node = meta
        .resolve
        .nodes
        .iter()
        .find(|n| n.id == app_pkg.id)
        .unwrap_or_else(|| panic!("resolve node not found for app pkg id: {}", app_pkg.id));

    let mut out = Vec::new();
    for dep in &node.deps {
        if !dep_kind_allowed(dep, intent) {
            continue;
        }

        let Some(pkg) = meta.packages.iter().find(|p| p.id == dep.pkg) else {
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

fn dep_kind_allowed(dep: &crate::CargoDep, intent: Intent) -> bool {
    // cargo metadata dep_kinds.kind:
    // - None => normal dependency
    // - Some("dev") => dev-dependency
    // - Some("build") => build-dependency
    // We only care about normal/dev as SoT.
    let mut has_normal = false;
    let mut has_dev = false;

    for k in &dep.dep_kinds {
        match k.kind.as_deref() {
            None => has_normal = true,
            Some("dev") => has_dev = true,
            Some(_) => {}
        }
    }

    match intent {
        Intent::Build => has_normal,
        Intent::Test => has_normal || has_dev,
    }
}

fn find_ridl_files(src_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(src_dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("ridl") {
            out.push(p);
        }
    }
    out.sort();
    out
}
