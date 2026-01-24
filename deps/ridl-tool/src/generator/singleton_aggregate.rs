use askama::Template;

use crate::plan::RidlPlan;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Template)]
#[template(path = "ridl_context_ext.rs.j2")]
struct RidlContextExtTemplate {
    slots: Vec<Slot>,
    slot_inits: Vec<SlotInit>,
}

#[derive(Debug, Clone)]
pub(super) struct ProtoVarInit {
    pub(super) class_id_ident: String,
    pub(super) field_name: String,
    pub(super) field_type: crate::parser::ast::Type,
    pub(super) init_literal: String,
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

pub(super) fn collect_proto_vars(
    plan: &RidlPlan,
) -> Result<Vec<ProtoVarInit>, Box<dyn std::error::Error>> {
    let mut proto_vars: Vec<ProtoVarInit> = Vec::new();

    for m in &plan.modules {
        for ridl_file in &m.ridl_files {
            let src = std::fs::read_to_string(ridl_file)?;
            let parsed = crate::parser::parse_ridl_file(&src)?;

            for item in &parsed.items {
                if let crate::parser::ast::IDLItem::Class(c) = item {
                    let module_name_up = parsed
                        .module
                        .as_ref()
                        .map(|m| sanitize_ident(&m.module_path).to_uppercase())
                        .unwrap_or_else(|| "GLOBAL".to_string());
                    let class_id_ident = format!(
                        "{}_{}",
                        module_name_up,
                        sanitize_ident(&c.name).to_uppercase()
                    );
                    for f in &c.js_fields {
                        if !f
                            .modifiers
                            .contains(&crate::parser::ast::PropertyModifier::Proto)
                        {
                            continue;
                        }
                        proto_vars.push(ProtoVarInit {
                            class_id_ident: class_id_ident.clone(),
                            field_name: f.name.clone(),
                            field_type: f.field_type.clone(),
                            init_literal: f.init_literal.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(proto_vars)
}

pub(super) fn generate_ridl_context_ext(
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

    // proto vars are installed by JS_RIDL_StdlibInit (generated into mquickjs_ridl_register.c).

    for m in &plan.modules {
        for ridl_file in &m.ridl_files {
            let src = std::fs::read_to_string(ridl_file)?;
            let parsed = crate::parser::parse_ridl_file(&src)?;

            for item in &parsed.items {
                match item {
                    crate::parser::ast::IDLItem::Singleton(s) => {
                        let name = sanitize_ident(&s.name);

                        // Singleton slot indices must be globally unique across modules.
                        // Use the same key as slot_inits (module_ns + singleton name),
                        // not just the singleton name.
                        let module_ns = parsed
                            .module
                            .as_ref()
                            .map(|m| m.module_path.as_str())
                            .unwrap_or("GLOBAL");
                        let module_ns = sanitize_ident(module_ns).to_lowercase();

                        let slot_key = format!("singleton_{}_{}", module_ns, name.to_lowercase());
                        let slot_index = *slot_map.entry(slot_key.clone()).or_insert_with(|| {
                            let next = slots.len() as u32;
                            slots.push(Slot {
                                name: slot_key.clone(),
                                index: next,
                            });
                            next
                        });

                        slot_inits.push(SlotInit {
                            crate_name: m.crate_name.clone(),
                            slot_index,
                            vt_ident: format!(
                                "RIDL_{}_CTX_SLOT_VT",
                                crate::generator::naming::to_snake_case(&name).to_uppercase()
                            ),
                            slot_key: slot_key.clone(),
                        });
                    }
                    crate::parser::ast::IDLItem::Class(c) => {
                        // Only generate proto backing when the class has at least one proto property.
                        let has_proto = c.properties.iter().any(|p| {
                            p.modifiers
                                .contains(&crate::parser::ast::PropertyModifier::Proto)
                        });
                        if !has_proto {
                            // Note: proto vars are installed on JS prototype, independent from proto state.
                            // We still need to collect them even when there's no proto state.
                        }

                        // proto vars are installed by JS_RIDL_StdlibInit (mquickjs_ridl_register.c),
                        // so ridl_context_ext.rs generation does not emit them.

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
                                crate::generator::naming::to_snake_case(&c.name).to_uppercase()
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

    for init in slot_inits.iter_mut() {
        if let Some(idx) = slot_map.get(&init.slot_key.to_lowercase()) {
            init.slot_index = *idx;
        }
    }

    // Sort after re-mapping to final indices.
    slot_inits.sort_by(|a, b| {
        (a.crate_name.as_str(), a.slot_index).cmp(&(b.crate_name.as_str(), b.slot_index))
    });

    // Ensure we never generate duplicate slot indices (would lead to unreachable match arms
    // and broken slot dispatch at runtime).
    {
        let mut used: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
        for init in slot_inits.iter() {
            if let Some(prev) = used.insert(init.slot_index, init.slot_key.clone()) {
                return Err(format!(
                    "duplicate ctx slot index {}: {} vs {}\nslot_inits={:?}\nslots={:?}",
                    init.slot_index, prev, init.slot_key, slot_inits, slots
                )
                .into());
            }
        }
    }

    let t = RidlContextExtTemplate { slots, slot_inits };
    std::fs::write(out_dir.join("ridl_context_ext.rs"), t.render()?)?;
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
