use std::path::PathBuf;

use crate::parser;

use super::{
    TemplateClass, TemplateFunction, TemplateInterface, TemplateModule, TemplateSingleton,
};

pub(super) fn build_template_modules(
    ridl_files: &[PathBuf],
    classes: &[TemplateClass],
) -> Result<Vec<TemplateModule>, Box<dyn std::error::Error>> {
    // This function mirrors the module parsing + allocation logic in generator/mod.rs,
    // but accepts the already-allocated TemplateClass list (from AggregateIR) as the
    // source of truth for class_id values.

    // Parse modules.
    let mut modules: Vec<TemplateModule> = Vec::new();

    for ridl_file in ridl_files {
        let content = std::fs::read_to_string(ridl_file)?;
        let parsed = parser::parse_ridl_file(&content)?;

        let module_name = parsed
            .module
            .as_ref()
            .map(|m| m.module_path.clone())
            .unwrap_or_else(|| "GLOBAL".to_string());
        let module_name_normalized = crate::generator::filters::normalize_ident(&module_name)
            .unwrap_or_else(|_| "GLOBAL".to_string());

        let mut interfaces: Vec<TemplateInterface> = Vec::new();
        let mut functions: Vec<TemplateFunction> = Vec::new();
        let mut singletons: Vec<TemplateSingleton> = Vec::new();
        let mut local_classes: Vec<TemplateClass> = Vec::new();

        for item in &parsed.items {
            match item {
                parser::ast::IDLItem::Function(f) => {
                    functions.push(TemplateFunction::from_with_mode(
                        f.clone(),
                        parsed.mode,
                        module_name_normalized.clone(),
                    ))
                }
                parser::ast::IDLItem::Interface(i) => {
                    interfaces.push(TemplateInterface::from_with_mode(
                        i.clone(),
                        parsed.mode,
                        module_name_normalized.clone(),
                    ))
                }
                parser::ast::IDLItem::Singleton(s) => {
                    // Singleton method glue is not generated here (slot dispatch
                    // lives in the per-module glue), but mquickjs_ridl_register.c
                    // needs the singleton (its plain `var` js_fields) to emit the
                    // JS_RIDL_StdlibInit field installation for GLOBAL mode.
                    singletons.push(TemplateSingleton::from_ast(
                        s.clone(),
                        module_name.clone(),
                        parsed.mode,
                    ));
                }
                parser::ast::IDLItem::Class(c) => {
                    local_classes.push(TemplateClass::from_with_mode(
                        module_name.clone(),
                        module_name_normalized.clone(),
                        c.clone(),
                        parsed.mode,
                    ))
                }
                _ => {}
            }
        }

        let (require_full_name, require_base, require_v_major, require_v_minor, require_v_patch) =
            if let Some(m) = &parsed.module {
                let ver = m
                    .version
                    .as_ref()
                    .expect("module version is required by parser");
                let v = crate::parser::require_spec::Version::parse_no_ws(ver)
                    .expect("module version is validated by parser");
                (
                    format!("{}@{}", m.module_path, ver),
                    m.module_path.clone(),
                    v.major,
                    v.minor,
                    v.patch,
                )
            } else {
                (String::new(), String::new(), 0, 0, 0)
            };

        modules.push(TemplateModule {
            module_name,
            module_decl: parsed.module,
            file_mode: parsed.mode,
            require_full_name,
            require_base,
            require_v_major,
            require_v_minor,
            require_v_patch,
            module_class_id: 0,
            interfaces,
            functions,
            singletons,
            classes: local_classes,
        });
    }

    // Assign module_class_id in stable order.
    let mut next_module_id: u32 = 0;
    for m in &mut modules {
        if m.module_decl.is_some() {
            m.module_class_id = next_module_id;
            next_module_id += 1;
        }
    }

    // Apply class_id allocation from AggregateIR classes.
    // We match by (module_name, class_name).
    for m in &mut modules {
        for c in &mut m.classes {
            if let Some(found) = classes
                .iter()
                .find(|x| x.module_name == c.module_name && x.name == c.name)
            {
                c.class_id = found.class_id;
            }
        }
    }

    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Type;

    /// 三构造点透传（Phase C.3）：build_template_modules 构造的
    /// TemplateSingleton 必须携带 singleton 的 plain var js_fields，
    /// mquickjs_ridl_register.c 才能生成字段安装代码。
    #[test]
    fn build_template_modules_carries_singleton_js_fields() {
        let dir = tempfile::tempdir().unwrap();
        let ridl_path = dir.path().join("fixture_singleton_var.ridl");
        std::fs::write(
            &ridl_path,
            "singleton Store {\n    var count: i32 = 42;\n    var title: string = \"hi\";\n    fn ping() -> i32;\n}\n",
        )
        .unwrap();

        let modules = build_template_modules(&[ridl_path], &[]).unwrap();
        assert_eq!(modules.len(), 1);
        let m = &modules[0];
        assert!(m.module_decl.is_none(), "fixture is GLOBAL mode");
        assert_eq!(m.singletons.len(), 1);

        let s = &m.singletons[0];
        assert_eq!(s.name, "Store");
        assert_eq!(s.methods.len(), 1);
        assert_eq!(s.js_fields.len(), 2, "js_fields must be carried through");

        let f0 = &s.js_fields[0];
        assert_eq!(f0.name, "count");
        assert_eq!(f0.field_type, Type::I32);
        assert_eq!(f0.init_literal, "42");
        assert!(!f0.is_proto);

        let f1 = &s.js_fields[1];
        assert_eq!(f1.name, "title");
        assert_eq!(f1.field_type, Type::String);
        assert_eq!(f1.init_literal, "hi");
    }
}
