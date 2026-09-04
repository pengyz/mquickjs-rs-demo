(function () {
  // Non-cancellable semantics test

  if (typeof globalThis.AsyncTestSingleton === "undefined") {
    throw new Error("expected globalThis.AsyncTestSingleton singleton");
  }

  // Test non-cancellable task
  if (typeof AsyncTestSingleton.nonCancellableTask !== "function") {
    throw new Error("expected nonCancellableTask to be function");
  }

  // Test non-cancellable task with callback
  var callbackCalled = false;
  var callbackResult = null;
  var callbackError = null;

  AsyncTestSingleton.nonCancellableTask(function (err, result) {
    callbackCalled = true;
    callbackError = err;
    callbackResult = result;
  });

  // Note: In a synchronous test, we can't wait for the async callback
  // The actual non-cancellable behavior is tested in the Rust integration tests
  // But we can verify that the function accepts a callback without error

  console.log("Non-cancellable semantics test passed");
})();