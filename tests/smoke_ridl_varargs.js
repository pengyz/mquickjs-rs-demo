// varargs smoke test
// - default mode is strong-typed (no implicit conversions)
// - strict mode forbids `any` except in variadic params (validated at build time)

// stdlib_demo2 provides sum_i32(...nums:int)->int and count_args(...args:any)->int

if (sum_i32(1, 2, 3) !== 6) throw new Error("sum_i32 should sum ints");
if (count_args(1, "x", true, { a: 1 }) !== 4) throw new Error("count_args should count args");

var ok = false;
try { sum_i32(1, "2"); } catch (err1) { ok = true; }
if (!ok) throw new Error("expected throw for bad types in varargs (string -> int)");

"ok";
