var m = require("test_module_basic@1.0");

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

assert(typeof m === "object", "module should be object");
assert(typeof m.MFoo === "function", "expected MFoo export");
assert(typeof m.MBar === "function", "expected MBar export");

var f = new m.MFoo();
assert(f.add(1, 2) === 3, "MFoo.add mismatch");

// v1-style mapping: snake_case
assert(typeof f.make_bar === "function", "expected MFoo.make_bar");
assert(typeof f.use_bar === "function", "expected MFoo.use_bar");

var b = f.make_bar(123);
assert(b instanceof m.MBar, "make_bar should return MBar instance");
assert(typeof b.get_v === "function", "expected MBar.get_v");
assert(b.get_v() === 123, "MBar.get_v mismatch");

// Cross-class parameter passing: pass MBar into MFoo
assert(f.use_bar(b) === 123, "MFoo.use_bar should return b.get_v() result");
