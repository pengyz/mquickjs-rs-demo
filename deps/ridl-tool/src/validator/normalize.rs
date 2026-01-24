use crate::parser::ast::{Function, Type, IDL};

pub(crate) fn ensure_default_constructors(idl: &mut IDL) {
    for c in &mut idl.classes {
        if c.constructor.is_none() {
            c.constructor = Some(Function {
                name: "constructor".to_string(),
                params: Vec::new(),
                return_type: Type::Void,
                is_async: false,
                module: None,
            });
        }
    }
}
