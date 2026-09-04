(function () {
    // 测试 ConstantTestSingleton 属性
    if (typeof globalThis.ConstantTestSingleton === "undefined") {
        throw new Error("expected globalThis.ConstantTestSingleton singleton");
    }

    // 测试 readonly 属性 - i32
    if (typeof ConstantTestSingleton.MAX_SIZE !== "number") {
        throw new Error("expected MAX_SIZE to be number, got " + typeof ConstantTestSingleton.MAX_SIZE);
    }
    if (ConstantTestSingleton.MAX_SIZE !== 100) {
        throw new Error("expected MAX_SIZE to be 100, got " + ConstantTestSingleton.MAX_SIZE);
    }

    // 测试 readonly 属性 - f64
    if (typeof ConstantTestSingleton.PI !== "number") {
        throw new Error("expected PI to be number, got " + typeof ConstantTestSingleton.PI);
    }
    if (Math.abs(ConstantTestSingleton.PI - 3.14159) > 0.00001) {
        throw new Error("expected PI to be ~3.14159, got " + ConstantTestSingleton.PI);
    }

    // 测试 readonly 属性 - string
    if (typeof ConstantTestSingleton.NAME !== "string") {
        throw new Error("expected NAME to be string, got " + typeof ConstantTestSingleton.NAME);
    }
    if (ConstantTestSingleton.NAME !== "test") {
        throw new Error("expected NAME to be 'test', got '" + ConstantTestSingleton.NAME + "'");
    }

    // 测试 readonly 属性 - bool
    if (typeof ConstantTestSingleton.ENABLED !== "boolean") {
        throw new Error("expected ENABLED to be boolean, got " + typeof ConstantTestSingleton.ENABLED);
    }
    if (ConstantTestSingleton.ENABLED !== true) {
        throw new Error("expected ENABLED to be true, got " + ConstantTestSingleton.ENABLED);
    }

    // 测试 read-write 属性 - i32
    if (typeof ConstantTestSingleton.counter !== "number") {
        throw new Error("expected counter to be number, got " + typeof ConstantTestSingleton.counter);
    }
    if (ConstantTestSingleton.counter !== 0) {
        throw new Error("expected counter to be 0, got " + ConstantTestSingleton.counter);
    }
    ConstantTestSingleton.counter = 42;
    if (ConstantTestSingleton.counter !== 42) {
        throw new Error("expected counter to be 42 after set, got " + ConstantTestSingleton.counter);
    }

    // 测试 read-write 属性 - string
    if (typeof ConstantTestSingleton.status !== "string") {
        throw new Error("expected status to be string, got " + typeof ConstantTestSingleton.status);
    }
    if (ConstantTestSingleton.status !== "idle") {
        throw new Error("expected status to be 'idle', got '" + ConstantTestSingleton.status + "'");
    }
    ConstantTestSingleton.status = "active";
    if (ConstantTestSingleton.status !== "active") {
        throw new Error("expected status to be 'active', got '" + ConstantTestSingleton.status + "'");
    }

    console.log("ConstantTestSingleton tests passed");
})();