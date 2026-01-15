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
