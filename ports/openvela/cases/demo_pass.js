/* Case corpus for the OpenVela sim (Phase 1, M1-C).
 *
 * Self-contained asserts: a thrown error = FAIL (sentinel FAIL: <msg>).
 * NOTE: no console here BY ARCHITECTURE — console.log belongs to the RIDL
 * stdlib layer (Phase 2b); the base engine variant exposes no globals beyond
 * the language builtins. The sentinel lines are the output channel.
 */

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

assert(1 + 1 === 2, "arithmetic");
assert(typeof "x" === "string", "typeof");
assert((function (a, b) { return a * b; })(6, 7) === 42, "call");

var obj = { name: "mqjs", n: 3 };
assert(obj.name === "mqjs" && obj.n === 3, "object fields");

var arr = [1, 2, 3];
assert(arr.length === 3 && arr[2] === 3, "array");

var caught = 0;
try { null.f(); } catch (e) { caught = 1; }
assert(caught === 1, "runtime TypeError caught");
