(function () {
  // Global class smoke + any? on class methods

  if (typeof globalThis.TestClass === "undefined") {
    throw new Error("expected globalThis.TestClass singleton");
  }

  var u = TestClass.makeUser("alice");
  if (!u) throw new Error("makeUser returned falsy");

  if (typeof u.getName !== "function") {
    throw new Error("expected User.getName to be function");
  }
  if (u.getName() !== "alice") {
    throw new Error("getName mismatch");
  }

  if (typeof u.echoAny !== "function") {
    throw new Error("expected User.echoAny to be function");
  }

  // None/null mapping
  var r0 = u.echoAny(null);
  if (r0 !== null) {
    throw new Error("echoAny(null) expected null, got " + (typeof r0));
  }

  // undefined treated as None as well
  var r1 = u.echoAny(undefined);
  if (r1 !== null) {
    throw new Error("echoAny(undefined) expected null, got " + (typeof r1));
  }

  // Some passthrough, keep value alive across allocation pressure
  var obj = { a: 1, b: "x" };
  var r2 = u.echoAny(obj);
  if (r2 !== obj) {
    throw new Error("echoAny(obj) expected same object identity");
  }

  // allocation pressure
  var keep = r2;
  for (var i = 0; i < 5000; i++) {
    var s = "x" + i + ":" + (i * 3);
    var o = { i: i, s: s };
    if (o.i !== i) throw new Error("alloc sanity");
  }
  if (keep !== obj || keep.a !== 1 || keep.b !== "x") {
    throw new Error("echoAny returned object corrupted after alloc pressure");
  }
})();
