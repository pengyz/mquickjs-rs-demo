(function () {
  // Async cancellation test

  if (typeof globalThis.AsyncTestSingleton === "undefined") {
    throw new Error("expected globalThis.AsyncTestSingleton singleton");
  }

  // Test non-cancellable task
  if (typeof AsyncTestSingleton.nonCancellableTask !== "function") {
    throw new Error("expected nonCancellableTask to be function");
  }

  // Test timeout task
  if (typeof AsyncTestSingleton.timeoutTask !== "function") {
    throw new Error("expected timeoutTask to be function");
  }

  // Test cancellable task
  if (typeof AsyncTestSingleton.cancellableTask !== "function") {
    throw new Error("expected cancellableTask to be function");
  }

  // Note: We can't easily test async callbacks in a synchronous test runner
  // The actual async behavior is tested in the Rust integration tests

  console.log("Async cancellation test passed");
})();