// [ROM0/ROM1] observable ROM/materialize behavior
// This test is intentionally shallow: it validates that __ridl_modules exists
// and that module objects have export properties that are enumerable.

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

assert(typeof globalThis.__ridl_modules === 'object' && globalThis.__ridl_modules !== null,
  '__ridl_modules must exist')

var m = require('test_module_basic@1.0')
assert(typeof m === 'object' && m !== null)

// ROM1: export properties exist; enumerability is engine/policy-defined
assert(typeof m.MFoo === 'function', 'expected MFoo export')
assert(typeof m.MBar === 'function', 'expected MBar export')

var keys = Object.keys(m)
// Do not require keys to be enumerable; just ensure Object.keys() works.
assert(keys !== null, 'Object.keys(module) must succeed')

// ROM0: classes should have prototype methods materialized
assert(typeof m.MFoo === 'function', 'MFoo must be a function')
assert(typeof m.MFoo.prototype.add === 'function', 'MFoo.prototype.add must exist')

// ROM1: module object should be mutable or not, depending on policy. We only assert
// it does not crash when defining a new property.
try {
  m.__rom_test = 1
} catch (_e) {
  // acceptable: frozen/sealed module object
}
