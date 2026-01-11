"use strict";

// Functions
if (default_echo_str("x") !== "x") throw new Error("default_echo_str failed");
if (default_add_i32(1, 2) !== 3) throw new Error("default_add_i32 failed");
if (default_not_bool(true) !== false) throw new Error("default_not_bool failed");
if (default_add_f64(0.5, 1.25) !== 1.75) throw new Error("default_add_f64 failed");

// any passthrough: just verify it returns *something* and doesn't throw.
// (We aren't asserting identity semantics yet in v1.)
var obj = { a: 1 };
var v = default_id_any(obj);
if (typeof v !== "object") throw new Error("default_id_any failed");

// void
default_void_ok();

// singleton
if (typeof demo !== "object") throw new Error("demo singleton missing");
if (typeof demo.ping !== "function") throw new Error("demo.ping missing");
demo.ping("hi");

// error paths (should throw)
var ok = false;
try { default_add_i32(1); } catch (err1) { ok = true; }
if (!ok) throw new Error("expected throw for missing arg");

var ok = false;
try { default_add_i32("1", "2"); } catch (err2) { ok = true; }
if (!ok) throw new Error("expected throw for bad types (string -> int)");

ok = false;
try { default_not_bool(1); } catch (err3) { ok = true; }
if (!ok) throw new Error("expected throw for bad types (number -> bool)");

ok = false;
try { default_echo_str(123); } catch (err4) { ok = true; }
if (!ok) throw new Error("expected throw for bad types (number -> string)");

"ok";
