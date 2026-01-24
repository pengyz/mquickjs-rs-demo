// Coverage extension for docs/planning/2026-01-24-module-basic-class-interop-test-matrix.md
// Focus: E0/E2 and export ordering invariants.

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

// Use versioned name to match harness preload behavior.
var m = require('test_module_basic@1.0')
assert(typeof m === 'object' && m !== null)

// E0/E1 sanity: at least one class export exists
assert(typeof m.MFoo === 'function', 'expected MFoo export')
assert(typeof m.MBar === 'function', 'expected MBar export')

// E2: module function + class mixed exports
assert(typeof m.mping === 'function', 'expected mping export')
assert(m.mping() === 7, 'mping mismatch')

// MC3: export order should not affect availability
// (in practice this catches accidental registration/order-dependent init bugs)
assert(typeof m.MFoo === 'function' && typeof m.MBar === 'function')

// instantiate both in either order
var b0 = new m.MBar()
var f0 = new m.MFoo()
assert(b0 instanceof m.MBar)
assert(f0 instanceof m.MFoo)
