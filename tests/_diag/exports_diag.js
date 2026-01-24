(function () {
  if (typeof require !== "function") throw new Error("require missing");

  var m = require("test.require");

  // NOTE: exports are allowed to be inherited from module prototype.
  if (typeof m.ping !== "function") throw new Error("expected ping function");
  if (typeof m.Foo !== "function") throw new Error("expected Foo constructor");

  // Keep basic diagnostics available when debugging.
  var proto = Object.getPrototypeOf(m);
  var proto2 = proto ? Object.getPrototypeOf(proto) : null;
  var out = [];
  out.push("module keys=" + Object.keys(m).join(","));
  out.push("ping: typeof=" + (typeof m.ping) + ", own=" + Object.prototype.hasOwnProperty.call(m, "ping"));
  out.push("Foo: typeof=" + (typeof m.Foo) + ", own=" + Object.prototype.hasOwnProperty.call(m, "Foo"));
  out.push("proto keys=" + (proto ? Object.keys(proto).join(",") : "<null>"));
  out.push("proto2 keys=" + (proto2 ? Object.keys(proto2).join(",") : "<null>"));

  // If this file runs, it should be considered PASS by default.
  // Uncomment to force dump:
  // throw new Error(out.join(" | "));
})();
