(function () {
  // module mode must not pollute globalThis with exported ctors
  if (typeof globalThis.MFoo !== "undefined") {
    throw new Error("unexpected global MFoo before require");
  }

  var m = require("test_module_basic@1.0");
  if (typeof m !== "object" || m === null) throw new Error("require should return object");

  if (typeof globalThis.MFoo !== "undefined") {
    throw new Error("module mode must not pollute globalThis.MFoo");
  }

  if (typeof m.mping !== "function") throw new Error("expected module function mping");
  if (m.mping() !== 7) throw new Error("mping mismatch");

  if (typeof m.MFoo !== "function") throw new Error("expected MFoo ctor export");
  var f = new m.MFoo();
  if (f.add(2, 3) !== 5) throw new Error("MFoo.add mismatch");
})();
