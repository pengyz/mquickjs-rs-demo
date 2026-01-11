"use strict";

if (typeof console === "undefined") {
  throw new Error("console is undefined");
}
if (typeof console.log !== "function") {
  throw new Error("console.log is not a function");
}

console.log("hi");

var ok = false;
try { console.log(123); } catch (err1) { ok = true; }
if (!ok) throw new Error("expected console.log(123) to throw in strict mode");

"ok";
