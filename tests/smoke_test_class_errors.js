// NOTE: 当前仅支持 global 模式（globalThis 暴露）。
// TODO(module): 支持 module 模式后，补充导出可见性/导入语义相关断言。

function assert(cond, msg) {
  if (!cond) throw new Error(msg || "assert failed");
}

function assertThrows(fn, msg) {
  var ok = false;
  try {
    fn();
  } catch (e) {
    ok = true;
  }
  assert(ok, msg || "expected throw");
}

// invalid receiver for method
assert(typeof Receiver === "function", "Receiver should be a constructor (global-only)");

// call method with wrong receiver should throw
assertThrows(function () {
  Receiver.prototype.get_tag.call({});
}, "invalid receiver should throw");

// missing args type errors
assert(typeof Basic === "function", "Basic should be a constructor");
var b = new Basic();
assertThrows(function () {
  b.add(1);
}, "missing args should throw");
