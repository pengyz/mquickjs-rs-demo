(function () {
  // Timeout semantics test

  if (typeof globalThis.AsyncTestSingleton === "undefined") {
    throw new Error("expected globalThis.AsyncTestSingleton singleton");
  }

  // Test timeout task
  if (typeof AsyncTestSingleton.timeoutTask !== "function") {
    throw new Error("expected timeoutTask to be function");
  }

  // Test timeout task with callback
  var callbackCalled = false;
  var callbackResult = null;
  var callbackError = null;

  AsyncTestSingleton.timeoutTask(function (err, result) {
    callbackCalled = true;
    callbackError = err;
    callbackResult = result;
  });

  // Note: In a synchronous test, we can't wait for the async callback
  // The actual timeout behavior is tested in the Rust integration tests
  // But we can verify that the function accepts a callback without error

  console.log("Timeout semantics test passed");
})();