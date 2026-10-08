// C.4 e2e: GLOBAL singleton plain var 字段安装（Phase C）。
//
// 断言顺序即安装时序断言：
// 1) 在调用任一 singleton 方法之前，字段已经可读（字段由
//    JS_RIDL_StdlibInit 在用户脚本前安装）；
// 2) 值与类型正确，escaped 精确；
// 3) 字段 writable（赋新值读回）；
// 4) 最后调方法，确认方法分派不受字段安装影响。

var t = globalThis.TestSingletonVar;
if (typeof t === "undefined") throw new Error("TestSingletonVar singleton must exist");

// --- 1) 先于任何方法调用的字段读取 ---
if (typeof t.count !== "number") throw new Error("count: expected number, got " + typeof t.count);
if (t.count !== 42) throw new Error("count init mismatch: " + t.count);

if (typeof t.flag !== "boolean") throw new Error("flag: expected boolean, got " + typeof t.flag);
if (t.flag !== true) throw new Error("flag init mismatch: " + t.flag);

if (typeof t.title !== "string") throw new Error("title: expected string, got " + typeof t.title);
if (t.title !== "hello") throw new Error("title init mismatch: " + t.title);

if (t.nothing !== null) throw new Error("nothing init mismatch: " + t.nothing);

// --- 2) escaped 精确值（\" 解码 -> C 转义再生成） ---
if (t.escaped !== 'he said "hi"') throw new Error("escaped init mismatch: " + t.escaped);

// --- 3) writable：赋新值读回 ---
t.count = 100;
if (t.count !== 100) throw new Error("count should be writable, got " + t.count);

t.flag = false;
if (t.flag !== false) throw new Error("flag should be writable, got " + t.flag);

t.title = "changed";
if (t.title !== "changed") throw new Error("title should be writable, got " + t.title);

t.escaped = "rewritten";
if (t.escaped !== "rewritten") throw new Error("escaped should be writable, got " + t.escaped);

t.nothing = 0;
if (t.nothing !== 0) throw new Error("nothing should be writable, got " + t.nothing);

// --- 4) 方法分派不受字段安装影响 ---
var d = t.describe();
if (d !== "singleton-var-fields-ok") throw new Error("describe mismatch: " + d);
