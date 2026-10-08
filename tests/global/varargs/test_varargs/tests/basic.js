function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

function assertEq(a, b, msg) {
  if (a !== b) throw new Error(msg || ('expected ' + a + ' === ' + b))
}

function repeat(s, n) {
  var out = ''
  while (n-- > 0) out += s
  return out
}

var t = TestVarargs
assert(t, 'TestVarargs singleton must exist')

// Multi-element join: each element is collected as an owned String.
assertEq(t.joinAll('-', 'a', 'b', 'c'), 'a-b-c')

// Empty varargs: no elements, no separator.
assertEq(t.joinAll('-'), '')

// Single element: separator never inserted.
assertEq(t.joinAll('-', 'x'), 'x')

// Empty-string elements are preserved (inline short-string path, len 0).
assertEq(t.joinAll('-', '', ''), '-')

// 1-char parts exercise the engine's inline short-string path: JS_ToCString
// serves these from the caller's stack buffer, so the glue must copy each
// element before the buffer is reused by the next iteration.
assertEq(t.joinAll('', 'a', 'b', 'c'), 'abc')

// Long parts exercise the GC-heap string path (multi-char strings are
// heap JSStrings owned by the tracing GC).
var p1 = repeat('abcdefghij', 10) // 100 chars
var p2 = repeat('0123456789', 10) // 100 chars
assertEq(p1.length, 100)
assertEq(p2.length, 100)
{
  var r = t.joinAll('|', p1, p2)
  assertEq(r, p1 + '|' + p2)
  assertEq(r.length, 201)
}

// Allocation pressure between calls: collected Strings are owned Rust data,
// so GC activity must not corrupt already-collected or later results.
{
  var expect = p1 + '|' + p2
  var sink = []
  for (var i = 0; i < 2000; i++) sink.push({ i: i, s: 'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz' + i })
  assertEq(t.joinAll('|', p1, p2), expect)
  assertEq(t.joinAll('-', 'a', 'b', 'c'), 'a-b-c')
}

// TypeError cases (RIDL is strict: no implicit conversions on varargs).
var threw = false
try {
  t.joinAll('-', 1)
} catch (e1) {
  threw = true
}
assert(threw, 'expected TypeError for non-string vararg joinAll("-", 1)')

threw = false
try {
  t.joinAll(1, 'a')
} catch (e2) {
  threw = true
}
assert(threw, 'expected TypeError for non-string sep joinAll(1, "a")')

// Missing fixed param must throw (varargs cannot satisfy sep).
threw = false
try {
  t.joinAll()
} catch (e3) {
  threw = true
}
assert(threw, 'expected TypeError for joinAll() with missing sep')

'ok'
