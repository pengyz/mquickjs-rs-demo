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

  var m1 = require("test.require");
  var m2 = require("test.require");
  if (m1 === m2) throw new Error("require must return a new instance each time");

  // Debug: ensure the created object uses the expected module class id.
  if (m1.__ridl_class_id !== m1.__ridl_expected_class_id) {
    throw new Error("module class id mismatch: got=" + m1.__ridl_class_id + " expected=" + m1.__ridl_expected_class_id);
  }


  if (typeof m1.ping !== "function") throw new Error("expected ping function");

  var Foo = m1.Foo;
  if (typeof Foo !== "function") throw new Error("expected Foo constructor");

  var foo = new Foo();
  if (typeof foo.value !== "function") throw new Error("expected Foo.value method");
})();
