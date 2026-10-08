// Phase D (TODO 3a): struct/enum 命名类型端到端 JS 用例。
//
// 覆盖：struct 返回对象字段断言、echo 透传（含嵌套 struct 与 array）、
// 缺失字段 strict 报错、多余字段忽略、字段类型错误报错、enum 双向映射
// ('RED' -> 'GREEN')、非法/非字符串变体名抛错、分配压力后对象身份。

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed')
}

function assertEq(a, b, msg) {
  if (a !== b) throw new Error(msg || ('expected ' + a + ' === ' + b))
}

function assertThrows(fn, msg, msgPart) {
  try {
    fn()
  } catch (e) {
    if (msgPart !== undefined && ('' + e).indexOf(msgPart) < 0) {
      throw new Error(msg || ('expected error containing "' + msgPart + '", got: ' + e))
    }
    return
  }
  throw new Error(msg || 'expected an exception')
}

var t = TestStructEnum
assert(t, 'TestStructEnum singleton must exist')

// ---------------------------------------------------------------------------
// struct 返回：字段注入
// ---------------------------------------------------------------------------

{
  var a = t.makeAddress('main st', 5)
  assertEq(a.street, 'main st', 'returned object must carry street')
  assertEq(a.num, 5, 'returned object must carry num')
}

// ---------------------------------------------------------------------------
// struct 参数 + 返回：echo 透传（含嵌套 struct 与 array<string>）
// ---------------------------------------------------------------------------

{
  var p = t.makePerson({
    name: 'ada',
    address: { street: 'oak ave', num: 7 },
    tags: ['x', 'y', 'z'],
  })
  assertEq(p.name, 'ada', 'person name must roundtrip')
  assertEq(p.address.street, 'oak ave', 'nested struct street must roundtrip')
  assertEq(p.address.num, 7, 'nested struct num must roundtrip')
  assertEq(p.tags.length, 3, 'array field length must roundtrip')
  assertEq(p.tags[0], 'x', 'array field element 0 must roundtrip')
  assertEq(p.tags[2], 'z', 'array field element 2 must roundtrip')
}

{
  // echoAddress 透传：对象 -> Rust -> 对象
  var a2 = t.echoAddress({ street: 'echo st', num: 42 })
  assertEq(a2.street, 'echo st')
  assertEq(a2.num, 42)
}

// ---------------------------------------------------------------------------
// 缺失字段：strict 报错
// ---------------------------------------------------------------------------

assertThrows(
  function () {
    t.echoAddress({ street: 's' })
  },
  'missing num field must throw',
  'missing field'
)
assertThrows(
  function () {
    t.makePerson({ name: 'n' })
  },
  'missing nested/array fields must throw',
  'missing field'
)

// ---------------------------------------------------------------------------
// 多余字段：忽略
// ---------------------------------------------------------------------------

{
  var a3 = t.echoAddress({ street: 'ok st', num: 1, extra: 'ignored', more: 99 })
  assertEq(a3.street, 'ok st', 'extra fields must be ignored')
  assertEq(a3.num, 1)
}

// ---------------------------------------------------------------------------
// 字段类型错误：报错
// ---------------------------------------------------------------------------

assertThrows(
  function () {
    t.echoAddress({ street: 5, num: 1 })
  },
  'wrong-typed field must throw',
  'invalid string argument'
)
assertThrows(
  function () {
    t.echoAddress({ street: 's', num: 'not-a-number' })
  },
  'non-number num field must throw',
  'invalid i32 argument'
)
assertThrows(
  function () {
    t.echoAddress(42)
  },
  'non-object argument must throw',
  "struct 'Address': expected object"
)
assertThrows(
  function () {
    t.makePerson({ name: 'n', address: { street: 's', num: 1 }, tags: 'not-an-array' })
  },
  'non-array tags field must throw',
  'expected array'
)

// ---------------------------------------------------------------------------
// enum：双向映射（JS 字符串 = RIDL 原始变体名）
// ---------------------------------------------------------------------------

assertEq(t.nextColor('RED'), 'GREEN', 'RED must map to GREEN')
assertEq(t.nextColor('GREEN'), 'BLUE', 'GREEN must map to BLUE')
assertEq(t.nextColor('BLUE'), 'RED', 'BLUE must map to RED')
assertEq(t.echoColor('RED'), 'RED', 'echoColor must roundtrip raw variant name')
assertEq(t.echoColor('BLUE'), 'BLUE')

// 分配压力后枚举映射保持正确
{
  var sink = []
  for (var i = 0; i < 2000; i++) sink.push({ i: i, s: 'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz' + i })
  assertEq(t.nextColor('RED'), 'GREEN', 'enum mapping must survive GC pressure')
}

// ---------------------------------------------------------------------------
// enum：非法/非字符串变体名抛错
// ---------------------------------------------------------------------------

assertThrows(
  function () {
    t.nextColor('PURPLE')
  },
  'unknown variant must throw',
  'unknown variant'
)
assertThrows(
  function () {
    t.nextColor('red')
  },
  'variant names are case-sensitive raw names',
  'unknown variant'
)
assertThrows(
  function () {
    t.nextColor(1)
  },
  'non-string enum argument must throw',
  'expected string'
)

// ---------------------------------------------------------------------------
// 分配压力后对象身份：早先返回的 struct 对象不被破坏
// ---------------------------------------------------------------------------

{
  var early = t.makePerson({
    name: 'early',
    address: { street: 'stable st', num: 123456 },
    tags: ['keep', 'me'],
  })
  var sink2 = []
  for (var j = 0; j < 2000; j++) {
    sink2.push(t.echoAddress({ street: 'pressure' + j, num: j }))
  }
  assertEq(early.name, 'early', 'early object must survive allocation pressure')
  assertEq(early.address.street, 'stable st')
  assertEq(early.address.num, 123456)
  assertEq(early.tags[0], 'keep')
  assertEq(early.tags[1], 'me')
  assertEq(sink2[0].street, 'pressure0', 'stored echoes must stay valid')
  assertEq(sink2[0].num, 0)
  assertEq(sink2[1999].street, 'pressure1999')
  assertEq(sink2[1999].num, 1999)
}

console.log('struct_enum basic: all assertions passed')
