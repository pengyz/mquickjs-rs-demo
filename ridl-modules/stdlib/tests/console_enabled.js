"use strict";

if (typeof console === "undefined") {
  throw new Error("console is undefined");
}

if (typeof console.enabled !== "boolean") {
  throw new Error("console.enabled should be boolean");
}

"ok";
