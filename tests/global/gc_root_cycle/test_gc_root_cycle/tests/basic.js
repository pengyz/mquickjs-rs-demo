const { TestGc } = require('test_gc_root_cycle');

function gc() {
  // Engine exposes JS_GC() via globalThis.gc in this repo's test harness.
  // If not present, this test should be updated accordingly.
  if (typeof globalThis.gc !== 'function') {
    throw new Error('missing globalThis.gc()');
  }
  globalThis.gc();
  globalThis.gc();
}

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assert failed');
}

const node = TestGc.makeNode();
let obj = {};

node.setHeld(obj);
node.makeCycle();

// Drop JS-side refs.
obj = null;

// While node is still reachable from JS, it must not be finalized.
const before = node.finalizerCount();
gc();
assert(node.finalizerCount() === before, 'node unexpectedly finalized while reachable');

// Remove all native roots (held + self obj), then drop node JS reference.
node.dropAll();

// Drop last JS ref.
// eslint-disable-next-line no-unused-vars
let dropped = node;
// @ts-ignore
// (force drop)
dropped = null;

// Now it should be collectable.
gc();

// Can't call node.finalizerCount() after dropping it; instead we create a new node and
// check the global finalizer counter.
const node2 = TestGc.makeNode();
const after = node2.finalizerCount();
assert(after > before, 'expected finalizer count to increase after collection');
