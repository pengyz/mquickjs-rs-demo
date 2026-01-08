mod generated;

pub mod impls;

pub fn ensure_linked() {
    generated::symbols::ensure_symbols();
}

// Re-export glue symbols for C side registration / lookup if needed.
pub use generated::glue::*;
