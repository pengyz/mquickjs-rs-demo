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

assertEq(t.echoI32(0), 0)
assertEq(t.echoI32(123), 123)

{
  var v = 1.5
  var r = t.echoF64(v)
  assertNear(r, v, 1e-12)
}

{
  var v = 1.25
  var r = t.echoF32(v)
  // f32 roundtrip tolerance
  assertNear(r, v, 1e-6)
}

{
  // i64 safe integer range
  var v = 9007199254740991
  var r = t.echoI64(v)
  assertEq(r, v)
  var threw = false
  try {
    t.echoI64(9007199254740992)
  } catch (e) {
    threw = true
  }
  assert(threw, 'expected TypeError for echoI64(2^53)')
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

// Optional(i32)
assertEq(t.echoI32Nullable(null), null)
assertEq(t.echoI32Nullable(undefined), null)
assertEq(t.echoI32Nullable(123), 123)

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
  t.echoI32Nullable(1.5)
} catch (e2) {
  threw = true
}
assert(threw, 'expected TypeError for echoI32Nullable(1.5)')

// Union(string | i32)
assertEq(t.echoStringOrI32('hello'), 'hello')
assertEq(t.echoStringOrI32(123), 123)

threw = false
try {
  t.echoStringOrI32(1.5)
} catch (e3) {
  threw = true
}
assert(threw, 'expected TypeError for echoStringOrI32(1.5)')

// Optional(Union(string | i32))
assertEq(t.echoStringOrI32Nullable(null), null)
assertEq(t.echoStringOrI32Nullable(undefined), null)
assertEq(t.echoStringOrI32Nullable('hi'), 'hi')
assertEq(t.echoStringOrI32Nullable(456), 456)

threw = false
try {
  t.echoStringOrI32Nullable(1.5)
} catch (e4) {
  threw = true
}
assert(threw, 'expected TypeError for echoStringOrI32Nullable(1.5)')

// f64 + f64?
assertNear(t.echoF64New(1), 1, 1e-12)
assertNear(t.echoF64New(1.25), 1.25, 1e-12)

assertEq(t.echoF64NewNullable(null), null)
assertEq(t.echoF64NewNullable(undefined), null)
assertNear(t.echoF64NewNullable(2), 2, 1e-12)
assertNear(t.echoF64NewNullable(2.5), 2.5, 1e-12)

{
  var obj = { a: 1 }
  var ret = t.echoAny(obj)
  assert(ret === obj, 'echoAny(object) must preserve identity')
  assertEq(ret.a, 1)
}

// Optional(any) param + Optional(Union(string | i32)) return
assertEq(t.maybeUnionAny(null), null)
assertEq(t.maybeUnionAny(undefined), null)
assertEq(t.maybeUnionAny('hi'), 'hi')
{
  var out = t.maybeUnionAny(456)
  // engine numeric tagging may vary; accept either (i32) number or (f64) string fallback
  assert(out == 456, 'maybeUnionAny(i32) must be loosely 456')
}
// object handling is engine-defined for number/string coercion; just ensure it doesn't crash
assert(t.maybeUnionAny({}) !== undefined)

// map<i32, string>
assertEq(t.sumMapI32String({ "1": "a", "-2": "bb" }), 1 + 1 + (-2) + 2)
threw = false
try {
  t.sumMapI32String({ "1.2": "a" })
} catch (e5) {
  threw = true
}
assert(threw, 'expected TypeError for map<i32,_> invalid key')

// map<bool, i32>
assertEq(t.sumMapBoolI32({ "true": 10, "false": 20 }), 1 + 10 + 0 + 20)
threw = false
try {
  t.sumMapBoolI32({ "True": 1 })
} catch (e6) {
  threw = true
}
assert(threw, 'expected TypeError for map<bool,_> invalid key')

// map<i64, i32> (no safe-int restriction)
assertEq(t.sumMapI64I32({ "9007199254740993": 1 }), 993 + 1)

// return map<i32, string>
{
  var m = t.makeMapI32String()
  assertEq(m["1"], "a")
  assertEq(m["-2"], "bb")
}
