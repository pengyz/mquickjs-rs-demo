use crate::plan::RidlPlan;
use askama::Template;
use std::collections::BTreeMap;
use std::path::Path;


#[derive(Template)]
#[template(path = "ridl_runtime_support.rs.j2")]
struct RidlRuntimeSupportTemplate {
    slots: Vec<Slot>,
    slot_inits: Vec<SlotInit>,
}

#[derive(Debug, Clone)]
struct SlotInit {
    crate_name: String,
    slot_index: u32,
    vt_ident: String,
    slot_key: String,
}


#[derive(Debug, Clone)]
struct Slot {
    /// sanitized Rust identifier
    name: String,
    index: u32,
}

pub fn generate_ridl_runtime_support(
    plan: &RidlPlan,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // We share the same slot ordering and singleton init ordering with the legacy generator.
    // Slot indices are global within an app aggregate.
    let mut slots: Vec<Slot> = Vec::new();
    let mut slot_inits: Vec<SlotInit> = Vec::new();
    // Temporary: used for mapping slot_key -> final sorted index (filled after sorting).
    let mut slot_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut proto_keys: BTreeMap<String, String> = BTreeMap::new();

    for m in &plan.modules {
        for ridl_file in &m.ridl_files {
            let src = std::fs::read_to_string(ridl_file)?;
            let parsed = crate::parser::parse_ridl_file(&src)?;

            for item in parsed.items {
                match item {
                    crate::parser::ast::IDLItem::Singleton(s) => {
                        let name = sanitize_ident(&s.name);
                        let name_key = name.to_lowercase();

                        let slot_index = *slot_map.entry(name_key.clone()).or_insert_with(|| {
                            let next = slots.len() as u32;
                            slots.push(Slot {
                                name: name.clone(),
                                index: next,
                            });
                            next
                        });

                        let module_ns = parsed
                            .module
                            .as_ref()
                            .map(|m| m.module_path.as_str())
                            .unwrap_or("GLOBAL");
                        let module_ns = sanitize_ident(module_ns).to_lowercase();

                        slot_inits.push(SlotInit {
                            crate_name: m.crate_name.clone(),
                            slot_index,
                            vt_ident: format!("RIDL_{}_CTX_SLOT_VT", name.to_uppercase()),
                            slot_key: format!("singleton_{}_{}", module_ns, name.to_lowercase()),
                        });
                    }
                    crate::parser::ast::IDLItem::Class(c) => {
                        // Only generate proto backing when the class has at least one proto property.
                        let has_proto = c
                            .properties
                            .iter()
                            .any(|p| p.modifiers.contains(&crate::parser::ast::PropertyModifier::Proto));
                        if !has_proto {
                            continue;
                        }

                        // module_ns: prefer RIDL module declaration; fallback to crate name.
                        let module_ns = parsed
                            .module
                            .as_ref()
                            .map(|m| m.module_path.as_str())
                            .unwrap_or_else(|| m.crate_name.as_str());

                        let crate_ns_norm = sanitize_ns(module_ns);
                        let key = format!("proto:{}::{}", crate_ns_norm, c.name);

                        // Avoid duplicate proto keys across modules.
                        // Even though proto state is now a generic ctx slot, the same semantic
                        // (proto:<module>::<class>) should not be defined twice.
                        if let Some(prev) = proto_keys.get(&key) {
                            return Err(format!(
                                "duplicate proto key {key}: {prev} vs {}",
                                ridl_file.display()
                            )
                            .into());
                        }
                        proto_keys.insert(key.clone(), ridl_file.display().to_string());

                        let field_name = format!(
                            "proto_{}_{}",
                            sanitize_ident(&crate_ns_norm.replace('.', "_")).to_lowercase(),
                            sanitize_ident(&c.name).to_lowercase()
                        );

                        // Proto backings are stored as ctx-ext slots (same mechanism as singletons).
                        // We reserve a dedicated slot index using the field_name. We can only rely
                        // on the "existing entry" case *after* the final slot_map rebuild below,
                        // so here we always allocate a fresh slot.
                        let field_key = field_name.to_lowercase();
                        let slot_index = *slot_map.entry(field_key).or_insert_with(|| {
                            let next = slots.len() as u32;
                            slots.push(Slot {
                                name: field_name.clone(),
                                index: next,
                            });
                            next
                        });

                        // Proto backing is just another ctx slot init.
                        let module_name = c
                            .module
                            .as_ref()
                            .map(|m| sanitize_ident(&m.module_path).to_uppercase())
                            .unwrap_or_else(|| "GLOBAL".to_string());

                        slot_inits.push(SlotInit {
                            crate_name: m.crate_name.clone(),
                            slot_index,
                            vt_ident: format!(
                                "RIDL_{}_{}_PROTO_CTX_SLOT_VT",
                                module_name,
                                sanitize_ident(&c.name).to_uppercase()
                            ),
                            slot_key: field_name,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    slots.sort_by(|a, b| a.name.cmp(&b.name));
    // Rebuild slot_map with final sorted indices.
    slot_map.clear();
    for (i, s) in slots.iter_mut().enumerate() {
        s.index = i as u32;
        slot_map.insert(s.name.to_lowercase(), s.index);
    }

    slot_inits.sort_by(|a, b| {
        (a.crate_name.as_str(), a.slot_index).cmp(&(b.crate_name.as_str(), b.slot_index))
    });

    for init in slot_inits.iter_mut() {
        if let Some(idx) = slot_map.get(&init.slot_key.to_lowercase()) {
            init.slot_index = *idx;
        }
    }

    // Ensure we never generate duplicate slot indices (would lead to unreachable match arms
    // and broken slot dispatch at runtime).
    {
        let mut used: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
        for init in slot_inits.iter() {
            if let Some(prev) = used.insert(init.slot_index, init.slot_key.clone()) {
                return Err(format!(
                    "duplicate ctx slot index {}: {} vs {}",
                    init.slot_index, prev, init.slot_key
                )
                .into());
            }
        }
    }

    let t = RidlRuntimeSupportTemplate { slots, slot_inits };
    std::fs::write(out_dir.join("ridl_runtime_support.rs"), t.render()?)?;
    Ok(())
}


fn sanitize_ident(name: &str) -> String {
    // Keep it simple: allow [A-Za-z_][A-Za-z0-9_]*; otherwise map to underscores.
    // Also avoid Rust keywords minimally.
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        let ok = if i == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        };
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        out.push_str("ident");
    }

    // Avoid keywords we care about.
    match out.as_str() {
        "type" | "match" | "mod" | "crate" | "self" | "super" => format!("{out}_"),
        _ => out,
    }
}

fn sanitize_ns(module_ns: &str) -> String {
    // Key namespace is restricted to ASCII: [0-9A-Za-z_:.]
    // Replace any other char with '_'.
    module_ns
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == ':' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}
