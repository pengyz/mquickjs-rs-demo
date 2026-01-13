use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct BuildProfile {
    pub app_manifest: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceBuildConfig {
    pub default: Option<String>,
    pub profiles: BTreeMap<String, BuildProfile>,
}

pub fn parse_mquickjs_build_toml(path: &Path) -> WorkspaceBuildConfig {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));

    // Minimal parser for our current mquickjs.build.toml shape.
    // Intentionally avoids adding new deps (toml) to the orchestrator.
    let mut cfg = WorkspaceBuildConfig {
        default: None,
        profiles: BTreeMap::new(),
    };

    let mut current_profile: Option<String> = None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let sec = &line[1..line.len() - 1];
            current_profile = None;
            if let Some(name) = sec.strip_prefix("profiles.") {
                current_profile = Some(name.trim().to_string());
                cfg.profiles
                    .entry(name.trim().to_string())
                    .or_insert(BuildProfile {
                        app_manifest: "Cargo.toml".to_string(),
                    });
            }
            continue;
        }

        if let Some((k, v)) = split_kv(line) {
            if current_profile.is_none() {
                if k == "default" {
                    cfg.default = Some(unquote(v));
                }
            } else {
                let p = current_profile.clone().unwrap();
                if k == "app_manifest" {
                    cfg.profiles
                        .entry(p)
                        .and_modify(|bp| bp.app_manifest = unquote(v))
                        .or_insert(BuildProfile {
                            app_manifest: unquote(v),
                        });
                }
            }
        }
    }

    cfg
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let (k, rest) = line.split_once('=')?;
    Some((k.trim(), rest.trim()))
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}
