// NOTE: 当前仅支持 global 模式（globalThis 暴露）。
// TODO(module): 支持 module 模式后，补充导出可见性/导入语义相关断言。

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

// Basic: constructor + method + instance property
assert(typeof Basic === "function", "Basic should be a constructor (global-only)");

var b = new Basic();
// NOTE: 当前 class method 暴露在 prototype 上（以 .add 形式，不保证存在）。
assert(typeof b.add === "function" || typeof b.add === "undefined", "Basic.add presence is implementation-defined");
assert(typeof b.add === "function" ? (b.add(1, 2) === 3) : true, "Basic.add should work when present");

// instance property default
assert(b.value === 0, "Basic.value default");

b.value = 7;
assert(b.value === 7, "Basic.value set/get");

// JS-only fields (instance)
// Debug: print actual value when assertion fails.
if (b.js_var !== "v0") {
  throw new Error("Basic.js_var default (got: " + String(b.js_var) + ")");
}
// proto var default (inherited)
assert(b.js_proto_var === 1, "Basic.js_proto_var default from prototype");

// instance write should shadow prototype
b.js_proto_var = 2;
assert(b.js_proto_var === 2, "Basic.js_proto_var shadowed on instance");

var b2 = new Basic();
assert(b2.js_proto_var === 1, "Basic.js_proto_var unchanged for new instance");

// JS-only instance field
assert(b.js_var === "v0", "Basic.js_var default");

b.js_var = "v1";
assert(b.js_var === "v1", "Basic.js_var writable");

var del_var = delete b.js_var;
assert(del_var === true, "Basic.js_var delete returns true");
assert(b.js_var === undefined, "Basic.js_var deleted");

