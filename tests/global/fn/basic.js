function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

function assertEq(a, b, msg) {
  if (a !== b) throw new Error(msg || ('expected ' + a + ' === ' + b))
}

function assertThrows(fn, msg) {
  var threw = false
  try {
    fn()
  } catch (e) {
    threw = true
  }
  if (!threw) throw new Error(msg || 'expected throw')
}

var t = globalThis.TestFn;
assert(t, 'TestFn singleton must exist')

// quick diagnostics if exports are missing
assertEq(typeof t.addInt, 'function', 'TestFn.addInt missing')
assertEq(typeof t.echoAny, 'function', 'TestFn.echoAny missing')
assertEq(typeof t.makeAnyString, 'function', 'TestFn.makeAnyString missing')
assertEq(typeof t.anyToString, 'function', 'TestFn.anyToString missing')
assertEq(typeof t.makeArrayWithLen, 'function', 'TestFn.makeArrayWithLen missing')
assertEq(typeof t.arrLen, 'function', 'TestFn.arrLen missing')
assertEq(typeof t.arrPush, 'function', 'TestFn.arrPush missing')
assertEq(typeof t.arrSet, 'function', 'TestFn.arrSet missing')
assertEq(typeof t.arrGet, 'function', 'TestFn.arrGet missing')

// primitive smoke
{
  var f = t.addInt;
  var r = f.call(t, 1, 2);
  assertEq(r, 3, 'addInt failed')
}

// any return (closure-based escape)
{
  // primitives
  assertEq(t.echoAny(null), null)
  assertEq(t.echoAny(true), true)
  assertEq(t.echoAny(false), false)
  assertEq(t.echoAny(1), 1)
  assertEq(t.echoAny(1.5), 1.5)
  assertEq(t.echoAny('hello'), 'hello')

  // identity
  var obj = { a: 1 }
  var ret = t.echoAny(obj)
  assert(ret === obj, 'echoAny(object) must preserve identity')
  assertEq(ret.a, 1)

  var arr = [1, 2]
  var arrRet = t.echoAny(arr)
  assert(arrRet === arr, 'echoAny(array) must preserve identity')
  assertEq(arrRet[0], 1)
}

// any return via rust-created values
{
  var s = t.makeAnyString('abc')
  assertEq(typeof s, 'string')
  assertEq(s, 'abc')
}

// GC / allocation pressure: returned any must remain valid
// NOTE: mquickjs uses tracing GC; we don't assume a gc() hook exists.
{
  var o = t.echoAny({ k: 1 })
  // Create allocation pressure.
  var sink = []
  for (var i = 0; i < 2000; i++) {
    sink.push({ i: i, s: 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' + i })
  }
  assertEq(o.k, 1, 'echoAny returned object must survive allocation pressure')

  var s2 = t.makeAnyString('pressure')
  for (var j = 0; j < 2000; j++) {
    sink.push('yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy' + j)
  }
  assertEq(s2, 'pressure', 'makeAnyString returned string must survive allocation pressure')
}

// any param validation
// Note: RIDL `any` means the callee decides how to interpret it.
// anyToString currently coerces via Env::get_string and does not throw.
{
  assertEq(t.anyToString('xyz'), 'xyz')
  assertEq(t.anyToString(123), '123')
  assertEq(t.anyToString(null), 'null')
}

// array behavior (no policy enforced by RIDL; we let QuickJS decide)
{
  var a = t.makeArrayWithLen(2);
  assertEq(t.arrLen(a), 2, 'arrLen after makeArrayWithLen failed')

  assertEq(t.arrPush(a, 1), 3, 'arrPush returns new len failed')
  assertEq(t.arrLen(a), 3, 'arrLen after arrPush failed')

  // within-bounds write
  t.arrSet(a, 0, 42)
  assertEq(t.arrGet(a, 0), 42, 'arrSet/arrGet within bounds failed')

  // append at len
  t.arrSet(a, 3, 7)
  assertEq(t.arrGet(a, 3), 7, 'arrSet append failed')

  // out-of-bounds set: semantics are engine-defined; just ensure we can observe something sensible.
  // If engine creates holes, arrLen may grow; if engine rejects, arrLen may stay.
  var beforeLen = t.arrLen(a)
  t.arrSet(a, 10, 1)
  var afterLen = t.arrLen(a)
  assert(afterLen >= beforeLen, 'array length must not shrink')
  var v10 = t.arrGet(a, 10)
  // Either the value is set, or it's undefined/null depending on engine/array kind.
  assert(v10 === 1 || v10 === undefined || v10 === null, 'arrGet(10) must be observable')

  // negative index: should not crash; behavior is not specified.
  var len2 = t.arrLen(a)
  t.arrSet(a, -1, 9)
  assertEq(t.arrLen(a), len2, 'negative index should not change length')
  assertEq(t.arrGet(a, -1), undefined, 'arrGet(-1) should be undefined')
}

// error paths / coercions (spot-check)
{
  // anyToString: ToString coercion is allowed.
  assertEq(t.anyToString(NaN), 'NaN')
  assertEq(t.anyToString(Infinity), 'Infinity')

  // addInt: missing arg must throw (glue validates argc)
  assertThrows(function () { t.addInt(1) }, 'addInt missing arg must throw')
}
