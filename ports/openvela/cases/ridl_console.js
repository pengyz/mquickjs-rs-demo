/* M1-R+ acceptance: console.log must be served by the RIDL stdlib singleton
 * (Rust: ridl-modules/stdlib DefaultConsoleSingleton -> print!/println!,
 * dispatched through the adapter's ridl_context_init state), NOT by a C hook.
 *
 * The case itself passes when no assertion throws; the expected stdout line
 * "ridl console in openvela sim" (printed by the Rust console BEFORE the
 * sentinel line) is checked by the host-side runner.
 *
 * NOTE: console here exists only because the sim image's stdlib is the RIDL
 * variant (mqjs_stdlib_impl.c strong js_stdlib with JS_RIDL_EXTENSIONS).
 */

if (typeof console !== "object") throw new Error("console missing");
if (typeof console.log !== "function") throw new Error("console.log missing");
if (typeof console.error !== "function") throw new Error("console.error missing");
if (console.enabled !== true) {
  throw new Error("console.enabled unexpected: " + console.enabled);
}

console.log("ridl console in openvela sim");
