// Smoke test for multi RIDL modules: stdlib_demo + stdlib_demo2

var a = globalThis.sayHello;
var b = globalThis.sayHello2;

if (typeof a != 'function') {
  throw new Error('globalThis.sayHello is not a function');
}
if (typeof b != 'function') {
  throw new Error('globalThis.sayHello2 is not a function');
}

a() + b();
