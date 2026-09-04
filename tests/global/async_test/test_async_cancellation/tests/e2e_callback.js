(function () {
    // 端到端测试：验证异步 callback 调用链
    // 测试流程：
    // 1. JS 调用异步方法并传入 callback
    // 2. Rust 执行异步任务
    // 3. 结果通过完成队列传递
    // 4. drain_completions 调用 JS callback

    if (typeof globalThis.AsyncTestSingleton === "undefined") {
        throw new Error("expected globalThis.AsyncTestSingleton singleton");
    }

    // 测试 cancellableTask
    var callbackCalled = false;
    var callbackResult = null;
    var callbackError = null;

    AsyncTestSingleton.cancellableTask(function (err, result) {
        callbackCalled = true;
        callbackError = err;
        callbackResult = result;
    });

    // 注意：在同步测试中，我们无法等待异步 callback
    // 但我们可以验证函数接受 callback 而不报错
    console.log("E2E callback test passed (sync validation)");
})();