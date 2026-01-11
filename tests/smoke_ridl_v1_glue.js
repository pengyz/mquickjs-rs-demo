"use strict";

// Functions
if (echo_str("x") !== "x") throw new Error("echo_str failed");
if (add_i32(1, 2) !== 3) throw new Error("add_i32 failed");
if (not_bool(true) !== false) throw new Error("not_bool failed");
if (add_f64(0.5, 1.25) !== 1.75) throw new Error("add_f64 failed");

// any passthrough: just verify it returns *something* and doesn't throw.
// (We aren't asserting identity semantics yet in v1.)
var obj = { a: 1 };
var v = id_any(obj);
if (typeof v !== "object") throw new Error("id_any failed");

// void
void_ok();

// singleton
if (typeof demo !== "object") throw new Error("demo singleton missing");
if (typeof demo.ping !== "function") throw new Error("demo.ping missing");
demo.ping("hi");

// error paths (should throw)
var ok = false;
try { add_i32(1); } catch (err1) { ok = true; }
if (!ok) throw new Error("expected throw for missing arg");

// default mode: JS_IsNumber check is not enforced; JS_ToInt32 may accept strings.
if (add_i32("1", "2") !== 3) throw new Error("expected add_i32 to accept numeric strings in default mode");

ok = false;
try { not_bool(1); } catch (err3) { ok = true; }
if (!ok) throw new Error("expected throw for bad types (number -> bool)");

// default mode: JS_IsString check is not enforced; JS_ToCString may accept non-strings.
if (echo_str(123) !== "123") throw new Error("expected echo_str to stringify number in default mode");

"ok";
