//! Build wiring for the openvela adapter as a RIDL leaf application.
//!
//! Copies this app's RIDL aggregate Rust artifacts (`ridl_symbols.rs`,
//! `ridl_context_ext.rs`, `ridl_bootstrap.rs`) from
//! `<target-dir>/ridl/apps/mqjs_openvela_adapter/aggregate/` into `OUT_DIR`.
//! The aggregate must be generated beforehand:
//!
//! ```text
//! cargo run -p ridl-builder -- aggregate \
//!     --cargo-toml <repo>/ports/openvela/rust/Cargo.toml --intent build
//! ```
//!
//! (`ports/openvela/setup-sim.sh` runs that step before `cargo build`.)
//!
//! Deliberately NO `emit_native_stdlib_link()` here: this crate builds with
//! `MQJS_ENGINE_LINK=external` (.cargo/config.toml) — the engine C objects,
//! including the ridl-variant stdlib TU that defines `js_stdlib`, are compiled
//! into the openvela image at source level. Linking the host-built
//! `libmquickjs_stdlib_ridl.a` would define `js_stdlib` twice (both strong).
fn main() {
    mquickjs_ridl_glue::emit();
}
