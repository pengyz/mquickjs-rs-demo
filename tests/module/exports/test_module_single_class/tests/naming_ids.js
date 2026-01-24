// [N0/N1/N2] naming/id/path conventions (observable surface)
// NOTE: current RIDL syntax does not allow special chars in `module` decl.
// So we validate the *require spec* normalization behavior at runtime.

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

// Canonical RIDL module id here is: test_module_single_1_0@1.0
var m1 = require('test_module_single_1_0@1.0')
assert(typeof m1 === 'object' && m1 !== null)

// Current engine behavior: require() does NOT normalize module ids with '.'/'-'.
// Keep this test as a guard: explicitly document that these names are rejected.
var threw = false
try { require('test-module.single@1.0') } catch (_e1) { threw = true }
assert(threw, 'expected require(test-module.single@1.0) to throw')

threw = false
try { require('test-module.single') } catch (_e2) { threw = true }
assert(threw, 'expected require(test-module.single) to throw')
