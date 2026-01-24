var m = require("test_module_basic@1.0");

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

assert(typeof m === "object", "module should be object");

// B0/B1/R1: import + require interop
var m2 = require("test_module_basic");
assert(typeof m2 === "object", "require() should return module object");

// NOTE: the harness preloads `m` using a versioned name (e.g. test_module_basic@1.0),
// while this test calls require() without version. They may legitimately be different
// objects, so we assert export identity instead of module object identity.

assert(typeof m.MFoo === "function", "expected MFoo export");
assert(typeof m.MBar === "function", "expected MBar export");
assert(m.MFoo === m2.MFoo, "MFoo export should be identical across import/require");
assert(m.MBar === m2.MBar, "MBar export should be identical across import/require");

var f = new m.MFoo();
assert(f.add(1, 2) === 3, "MFoo.add mismatch");

// v1-style mapping: snake_case
assert(typeof f.make_bar === "function", "expected MFoo.make_bar");
assert(typeof f.use_bar === "function", "expected MFoo.use_bar");

var b = f.make_bar(123);
assert(b instanceof m.MBar, "make_bar should return MBar instance");
assert(typeof b.get_v === "function", "expected MBar.get_v");
assert(b.get_v() === 123, "MBar.get_v mismatch");

// MC2: same-name method across classes should not pollute prototypes
assert(typeof f.get_v === "function", "expected MFoo.get_v");
assert(f.get_v() === 777, "MFoo.get_v mismatch");

// Cross-class parameter passing: pass MBar into MFoo
assert(f.use_bar(b) === 123, "MFoo.use_bar should return b.get_v() result");
