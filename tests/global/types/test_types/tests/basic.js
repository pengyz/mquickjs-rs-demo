function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

function assertEq(a, b, msg) {
  if (a !== b) throw new Error(msg || ('expected ' + a + ' === ' + b))
}

function assertNear(a, b, eps, msg) {
  if (Math.abs(a - b) > eps) throw new Error(msg || ('expected |' + a + '-' + b + '| <= ' + eps))
}

var t = TestTypes
assert(t, 'TestTypes singleton must exist')

assertEq(t.echoBool(true), true)
assertEq(t.echoBool(false), false)

assertEq(t.echoInt(0), 0)
assertEq(t.echoInt(123), 123)

{
  var v = 1.5
  var r = t.echoDouble(v)
  assertNear(r, v, 1e-12)
}

assertEq(t.echoAny(null), null)
assertEq(t.echoAny(true), true)
assertEq(t.echoAny(false), false)
assertEq(t.echoAny(1), 1)
assertEq(t.echoAny(1.5), 1.5)
assertEq(t.echoAny('hello'), 'hello')

// Optional(any) return: null represents None
assertEq(t.maybeAny(false), null)
assertEq(t.maybeAny(true), 'ok')

// allocation pressure: returned any? must remain valid
{
  var s = t.maybeAny(true)
  var sink = []
  for (var i = 0; i < 2000; i++) sink.push({ i: i, s: 'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz' + i })
  assertEq(s, 'ok')
}

assertEq(t.echoString('hello'), 'hello')

// NUL is allowed; passed string is truncated at NUL when crossing into Rust.
assertEq(t.echoString('a\u0000b'), 'a')

// Optional(string)
assertEq(t.echoStringNullable(null), null)
assertEq(t.echoStringNullable(undefined), null)
assertEq(t.echoStringNullable('hi'), 'hi')

// Optional(int)
assertEq(t.echoIntNullable(null), null)
assertEq(t.echoIntNullable(undefined), null)
assertEq(t.echoIntNullable(123), 123)

// TypeError cases (RIDL is strict even in default mode)
var threw = false
try {
  t.echoStringNullable(123)
} catch (e1) {
  threw = true
}
assert(threw, 'expected TypeError for echoStringNullable(123)')

threw = false
try {
  t.echoIntNullable(1.5)
} catch (e2) {
  threw = true
}
assert(threw, 'expected TypeError for echoIntNullable(1.5)')

// Union(string | int)
assertEq(t.echoStringOrInt('hello'), 'hello')
assertEq(t.echoStringOrInt(123), 123)

threw = false
try {
  t.echoStringOrInt(1.5)
} catch (e3) {
  threw = true
}
assert(threw, 'expected TypeError for echoStringOrInt(1.5)')

// Optional(Union(string | int))
assertEq(t.echoStringOrIntNullable(null), null)
assertEq(t.echoStringOrIntNullable(undefined), null)
assertEq(t.echoStringOrIntNullable('hi'), 'hi')
assertEq(t.echoStringOrIntNullable(456), 456)

threw = false
try {
  t.echoStringOrIntNullable(1.5)
} catch (e4) {
  threw = true
}
assert(threw, 'expected TypeError for echoStringOrIntNullable(1.5)')

{
  var obj = { a: 1 }
  var ret = t.echoAny(obj)
  assert(ret === obj, 'echoAny(object) must preserve identity')
  assertEq(ret.a, 1)
}

// Optional(any) param + Optional(Union(string | int)) return
assertEq(t.maybeUnionAny(null), null)
assertEq(t.maybeUnionAny(undefined), null)
assertEq(t.maybeUnionAny('hi'), 'hi')
{
  var out = t.maybeUnionAny(456)
  // engine numeric tagging may vary; accept either (int) number or (double) string fallback
  assert(out == 456, 'maybeUnionAny(int) must be loosely 456')
}
// object handling is engine-defined for number/string coercion; just ensure it doesn't crash
assert(t.maybeUnionAny({}) !== undefined)
