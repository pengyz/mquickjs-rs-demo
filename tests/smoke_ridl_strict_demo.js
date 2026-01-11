// strict demo smoke test
// - strict mode: no implicit conversions for string/int/double
// - strict mode: any only allowed in variadic (validated at build time)

"use strict";

var ok = false;
try { strict_add_i32("1", "2"); } catch (e1) { ok = true; }
if (!ok) throw new Error("expected strict_add_i32 to throw for string inputs");

ok = false;
try { strict_echo_str(123); } catch (e2) { ok = true; }
if (!ok) throw new Error("expected strict_echo_str to throw for number input");

if (strict_count_args(1, "x", true, { a: 1 }) !== 4) throw new Error("strict_count_args failed");

"ok";
