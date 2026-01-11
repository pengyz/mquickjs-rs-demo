pub(crate) mod glue {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_demo_default_glue.rs"));
}

pub(crate) mod symbols {
    include!(concat!(env!("OUT_DIR"), "/ridl_module_demo_default_symbols.rs"));
}
