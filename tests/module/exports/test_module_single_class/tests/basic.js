// [E0] single class export + [N0/N2] module name normalization with dot/dash/version

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

var m = require('test_module_single_1_0')
assert(typeof m === 'object' && m !== null)

// E0: only one class export
assert(typeof m.Only === 'function', 'expected Only export')

// ensure no unexpected secondary class exports
assert(typeof m.MFoo === 'undefined', 'unexpected MFoo export')
assert(typeof m.MBar === 'undefined', 'unexpected MBar export')

var o = new m.Only()
assert(o instanceof m.Only, 'instanceof Only')
assert(o.getV() === 7, 'getV mismatch')
