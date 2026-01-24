// require() should exist only when ridl-extensions is enabled (this JS test suite
// is executed with ridl-extensions in CI for mquickjs-demo).

(function () {
  if (typeof require !== "function") {
    throw new Error("require should be a function when ridl-extensions is enabled");
  }

  // not found
  var ok = false;
  try {
    require("no.such.module");
  } catch (e) {
    ok = ("" + e).indexOf("require no.such.module failed: module not found.") >= 0;
  }
  if (!ok) throw new Error("expected module not found error");

  // sanity: built-in Object constructor must work (engine baseline)
  var o = new Object();
  if (typeof o !== "object") throw new Error("expected new Object() to yield object");
  if (Object.getPrototypeOf(o) !== Object.prototype) throw new Error("new Object() prototype mismatch");

  var m1 = require("test.require");
  var m2 = require("test.require");
  if (m1 === m2) throw new Error("require must return a new instance each time");

  // Debug: ensure the created object uses the expected module class id.
  if (m1.__ridl_class_id !== m1.__ridl_expected_class_id) {
    throw new Error("module class id mismatch: got=" + m1.__ridl_class_id + " expected=" + m1.__ridl_expected_class_id);
  }


  if (typeof m1.ping !== "function") throw new Error("expected ping function");

  function assertCtor(obj, name) {
    var C = obj[name];
    var tag = undefined;
    try { tag = Object.prototype.toString.call(C); } catch (_e) {}
    if (typeof C !== "function") {
      throw new Error(
        "expected " + name + " constructor, typeof=" + (typeof C) +
        ", toStringTag=" + String(tag)
      );
    }

    // Exports are allowed to be inherited from the module prototype.
    return C;
  }

  var Foo = assertCtor(m1, "Foo");
  var Bar = assertCtor(m1, "Bar");

  // matrix: direct vs indirect ctor call, and via globalThis
  globalThis.Foo = Foo;
  globalThis.Bar = Bar;

  var foo1 = new Foo();
  if (typeof foo1.value !== "function") throw new Error("expected Foo.value method (new Foo())");

  var foo2 = new globalThis.Foo();
  if (typeof foo2.value !== "function") throw new Error("expected Foo.value method (new globalThis.Foo())");

  var bar1 = new Bar();
  if (typeof bar1.value !== "function") throw new Error("expected Bar.value method (new Bar())");

  var bar2 = new globalThis.Bar();
  if (typeof bar2.value !== "function") throw new Error("expected Bar.value method (new globalThis.Bar())");

  // ensure different module instances can access ctors
  if (typeof m2.Foo !== "function") {
    throw new Error("expected m2.Foo constructor");
  }
  if (typeof m2.Bar !== "function") {
    throw new Error("expected m2.Bar constructor");
  }
})();
