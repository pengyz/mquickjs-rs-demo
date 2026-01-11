pub(crate) mod glue {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo2_glue.rs"));
}

pub(crate) mod symbols {
    include!(concat!(env!("OUT_DIR"), "/stdlib_demo2_symbols.rs"));
}
