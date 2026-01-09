// Smoke test for a single RIDL module: stdlib_demo
// Expected to have been registered by the build pipeline.

try {
  // Depending on how modules are exposed, either a global function exists
  // or the module is available via `std` / `globalThis`.
  const fn = globalThis.sayHello;
  if (typeof fn === 'function') {
    const r = fn();
    if (typeof r !== 'string' || r.length === 0) {
      throw new Error('sayHello() returned non-string or empty');
    }
    print('sayHello() =>', r);
  } else {
    // Fallback: just ensure it is at least defined somewhere
    // (keeps this script informative even if the exposure mechanism changes).
    throw new Error('globalThis.sayHello is not a function; module exposure differs');
  }

  print('stdlib_demo_smoke: OK');
} catch (e) {
  print('stdlib_demo_smoke: FAIL:', e && (e.stack || e));
  throw e;
}
