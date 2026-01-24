use std::collections::HashSet;

use super::ast::{Class, Function, IDLItem, Interface, Method, Param, Property, StructDef, Type};

pub fn rewrite_item_class_refs(item: &mut IDLItem, class_names: &HashSet<String>) {
    match item {
        IDLItem::Interface(Interface { methods, .. }) => {
            for m in methods {
                rewrite_method_class_refs(m, class_names);
            }
        }
        IDLItem::Class(Class {
            methods,
            properties,
            constructor,
            ..
        }) => {
            for m in methods {
                rewrite_method_class_refs(m, class_names);
            }
            for p in properties {
                rewrite_property_class_refs(p, class_names);
            }
            if let Some(ctor) = constructor {
                for Param { param_type, .. } in &mut ctor.params {
                    rewrite_type_class_refs(param_type, class_names);
                }
            }
        }
        IDLItem::Struct(StructDef { fields, .. }) => {
            for f in fields {
                rewrite_type_class_refs(&mut f.field_type, class_names);
            }
        }
        IDLItem::Function(Function { params, return_type, .. }) => {
            for Param { param_type, .. } in params {
                rewrite_type_class_refs(param_type, class_names);
            }
            rewrite_type_class_refs(return_type, class_names);
        }
        IDLItem::Singleton(s) => {
            for m in &mut s.methods {
                rewrite_method_class_refs(m, class_names);
            }
        }
        IDLItem::Enum(_) | IDLItem::Using(_) | IDLItem::Import(_) => {}
    }
}

fn rewrite_method_class_refs(method: &mut Method, class_names: &HashSet<String>) {
    for Param { param_type, .. } in &mut method.params {
        rewrite_type_class_refs(param_type, class_names);
    }
    rewrite_type_class_refs(&mut method.return_type, class_names);
}

fn rewrite_property_class_refs(prop: &mut Property, class_names: &HashSet<String>) {
    rewrite_type_class_refs(&mut prop.property_type, class_names);
}

fn rewrite_type_class_refs(ty: &mut Type, class_names: &HashSet<String>) {
    match ty {
        Type::Custom(name) if class_names.contains(name) => {
            *ty = Type::ClassRef(name.clone());
        }
        Type::Array(inner) | Type::Optional(inner) | Type::Group(inner) => {
            rewrite_type_class_refs(inner, class_names);
        }
        Type::Map(k, v) => {
            rewrite_type_class_refs(k, class_names);
            rewrite_type_class_refs(v, class_names);
        }
        Type::Union(types) => {
            for t in types {
                rewrite_type_class_refs(t, class_names);
            }
        }
        Type::CallbackWithParams(params) => {
            for Param { param_type, .. } in params {
                rewrite_type_class_refs(param_type, class_names);
            }
        }
        Type::ClassRef(_) => {}
        Type::Bool
        | Type::I32
        | Type::I64
        | Type::F32
        | Type::F64
        | Type::String
        | Type::Void
        | Type::Object
        | Type::Callback
        | Type::Null
        | Type::Any
        | Type::Custom(_) => {}
    }
}
