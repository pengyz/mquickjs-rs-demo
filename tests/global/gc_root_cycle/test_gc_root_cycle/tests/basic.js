// Smoke test: Node creation and method dispatch work.
// Uses ES5 syntax (mquickjs targets ES5 subset).

var TestGc = globalThis.TestGc;
if (typeof TestGc === "undefined") {
  throw new Error("expected globalThis.TestGc singleton");
}

var node = TestGc.makeNode();
var count = node.finalizerCount();
if (typeof count !== "number") {
  throw new Error("finalizerCount should return a number, got " + typeof count);
}

// Create a cycle with plain JS properties.
var obj = {};
node.held = obj;
obj.back = node;

// Verify method dispatch still works after forming cycle.
var count2 = node.finalizerCount();
if (count2 !== count) {
  throw new Error("finalizerCount should be stable, got " + count2 + " expected " + count);
}

// Drop references.
node = null;
obj.back = null;
obj = null;

// Note: mquickjs's JS_GC does NOT invoke class finalizers (by design).
// Finalizer-based verification is done in Rust tests at context teardown.
// This JS test only verifies creation and method dispatch.
