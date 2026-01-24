(function () {
  if (typeof globalThis.MFoo !== "undefined") {
    throw new Error("unexpected global MFoo before require");
  }

  var m = require("test_module_basic@1.0");

  var tag = function (v) {
    try { return typeof v; } catch (e) { return "<typeof threw>"; }
  };

  if (tag(m.MFoo) !== "function") {
    throw new Error("expected m.MFoo to be function, got " + tag(m.MFoo));
  }

  var a = new m.MFoo();
  var b = new m.MFoo();

  // avoid calling potentially missing builtins (hasOwnProperty) on this engine
  if (typeof a.x === "undefined") throw new Error("MFoo.x missing");
  if (typeof a.y === "undefined") throw new Error("MFoo.y missing");

  if (a.x !== 1) throw new Error("MFoo.x init mismatch");
  if (a.y !== "foo") throw new Error("MFoo.y init mismatch");

  a.x = 9;
  if (b.x !== 1) throw new Error("MFoo.x should be per-instance");

  a.y = "bar";
  if (a.y !== "bar") throw new Error("MFoo.y should be writable var");

  var proto = m.MFoo.prototype;
  if (!proto) throw new Error("MFoo.prototype missing");

  if (typeof proto.px === "undefined") {
    throw new Error("MFoo.prototype.px missing");
  }
  if (proto.px !== 100) throw new Error("MFoo.prototype.px init mismatch");
})();
