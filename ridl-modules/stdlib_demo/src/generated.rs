pub(crate) mod glue {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo_glue.rs"));
}

pub(crate) mod symbols {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo_symbols.rs"));
}
