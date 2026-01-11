"use strict";

if (typeof console === "undefined") {
  throw new Error("console is undefined");
}
if (typeof console.log !== "function") {
  throw new Error("console.log is not a function");
}

console.log("hi");

"ok";
