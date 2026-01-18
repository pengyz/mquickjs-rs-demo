var t = globalThis.TestFn;

// primitive smoke
var f = t.addInt;
var r = f.call(t, 1, 2);
if (r !== 3) {
  throw new Error("addInt failed");
}

// array(no-holes)
var a = t.makeArrayWithLen(2);
if (t.arrLen(a) !== 2) {
  throw new Error("arrLen after makeArrayWithLen failed");
}
if (t.arrPush(a, 1) !== 3) {
  throw new Error("arrPush returns new len failed");
}
if (t.arrLen(a) !== 3) {
  throw new Error("arrLen after arrPush failed");
}

t.arrSet(a, 10, 1);
if (t.arrLen(a) !== 3) {
  throw new Error("arrSet should not create holes or grow the array");
}
