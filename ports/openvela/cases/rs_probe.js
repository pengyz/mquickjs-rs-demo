if (typeof rsSelfTest !== "function") throw new Error("rsSelfTest missing");
if (rsSelfTest() !== 0) throw new Error("rsSelfTest failed: " + rsSelfTest());
if (typeof rsVersion !== "function") throw new Error("rsVersion missing");
var v = rsVersion();
if (typeof v !== "string" || v.length === 0) throw new Error("rsVersion bad: " + v);
console.log("rust-bridge ok: " + v);
