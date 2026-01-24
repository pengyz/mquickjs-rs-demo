// Coverage extension for docs/planning/2026-01-24-module-basic-class-interop-test-matrix.md
// Focus: C4 diagnostics around arity/type mismatch and nullability behavior.

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

var m = require('test_module_basic@1.0')
var f = new m.MFoo()

// arity mismatch: add expects 2 params
var threw = false
try { f.add(1) } catch (_e1) { threw = true }
assert(threw, 'expected throw for add(1)')

// type mismatch: echo_i32 expects i32
threw = false
try { f.echo_i32('x') } catch (_e2) { threw = true }
assert(threw, 'expected throw for echo_i32(string)')

// nullability mismatch: echo_string is non-nullable
threw = false
try { f.echo_string(null) } catch (_e3) { threw = true }
assert(threw, 'expected throw for echo_string(null)')

// f64 mismatch: echo_f64 should reject non-number
threw = false
try { f.echo_f64({}) } catch (_e4) { threw = true }
assert(threw, 'expected throw for echo_f64(object)')
