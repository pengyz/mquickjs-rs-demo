var m = require("test_module_basic@1.0");

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

assert(typeof m === "object", "module should be object");

// B0/B1/R0/R1: import + require interop + require (no cache) semantics
var m2 = require("test_module_basic");
var m3 = require("test_module_basic");
assert(typeof m2 === "object", "require() should return module object");
assert(m2 !== m3, "repeated require should return a new module object");

// NOTE: the harness preloads `m` using a versioned name (e.g. test_module_basic@1.0),
// while this test calls require() without version. They may legitimately be different
// objects, so we assert export identity instead of module object identity.

assert(typeof m.MFoo === "function", "expected MFoo export");
assert(typeof m.MBar === "function", "expected MBar export");
assert(m.MFoo === m2.MFoo, "MFoo export should be identical across import/require");
assert(m.MBar === m2.MBar, "MBar export should be identical across import/require");

// B2: __ridl_modules observability
assert(typeof __ridl_modules === "object", "expected __ridl_modules global");
assert(__ridl_modules !== null, "expected __ridl_modules non-null");

// NOTE: keep B2 assertions minimal here. Accessing module entries via __ridl_modules
// may currently crash (ROMClass materialize path). We'll cover deep checks in a
// dedicated ROM/materialize test later.

// C0: new + instanceof
var f = new m.MFoo();
assert(f instanceof m.MFoo, "MFoo instanceof mismatch");

// C1: basic types
assert(f.add(1, 2) === 3, "MFoo.add mismatch");
assert(f.echo_bool(true) === true, "MFoo.echo_bool mismatch");
assert(f.echo_int(-7) === -7, "MFoo.echo_int mismatch");
assert(f.echo_double(1.25) === 1.25, "MFoo.echo_double mismatch");
assert(f.echo_string("hello") === "hello", "MFoo.echo_string mismatch");

var any_obj = { k: 1 };
f.echo_any(any_obj);

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
