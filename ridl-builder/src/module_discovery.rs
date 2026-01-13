use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::aggregate::Module;

pub fn discover_ridl_modules(app_manifest: &Path) -> Vec<Module> {
    let base_dir = app_manifest
        .parent()
        .unwrap_or_else(|| panic!("app_manifest has no parent: {}", app_manifest.display()));

    let content = fs::read_to_string(app_manifest)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", app_manifest.display()));

    let mut in_deps = false;
    let mut modules = Vec::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Very small TOML subset parser: `name = { path = "..." }`
        let Some((name, rhs)) = line.split_once('=') else {
            continue;
        };
        let crate_name = name.trim().to_string();
        let rhs = rhs.trim();
        if !rhs.starts_with('{') {
            continue;
        }

        let path_val = extract_inline_table_string(rhs, "path")
            .map(|p| base_dir.join(p))
            .filter(|p| p.exists());

        let Some(crate_dir) = path_val else {
            continue;
        };

        let ridl_files = find_ridl_files(&crate_dir.join("src"));
        if ridl_files.is_empty() {
            continue;
        }

        modules.push(Module {
            crate_name,
            crate_dir,
            ridl_files,
        });
    }

    modules
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

fn extract_inline_table_string(rhs: &str, key: &str) -> Option<String> {
    // expects something like `{ path = "ridl-modules/stdlib" }`
    let needle = format!("{key}");
    let idx = rhs.find(&needle)?;
    let rest = &rhs[idx + needle.len()..];
    let eq = rest.find('=')?;
    let mut v = rest[eq + 1..].trim();
    // strip leading ',' or '{'
    if v.starts_with(',') {
        v = v[1..].trim();
    }
    // value ends at ',' or '}'
    let end = v
        .find(|c| c == ',' || c == '}')
        .unwrap_or_else(|| v.len());
    let v = v[..end].trim();
    Some(unquote(v))
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}
