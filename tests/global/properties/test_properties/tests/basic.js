(function () {
    // 测试 PropertyTestSingleton 属性
    if (typeof globalThis.PropertyTestSingleton === "undefined") {
        throw new Error("expected globalThis.PropertyTestSingleton singleton");
    }

    // 测试 readonly 属性 - bool
    if (typeof PropertyTestSingleton.boolProp !== "boolean") {
        throw new Error("expected boolProp to be boolean, got " + typeof PropertyTestSingleton.boolProp);
    }
    if (PropertyTestSingleton.boolProp !== true) {
        throw new Error("expected boolProp to be true, got " + PropertyTestSingleton.boolProp);
    }

    // 测试 readonly 属性 - i32
    if (typeof PropertyTestSingleton.i32Prop !== "number") {
        throw new Error("expected i32Prop to be number, got " + typeof PropertyTestSingleton.i32Prop);
    }
    if (PropertyTestSingleton.i32Prop !== 42) {
        throw new Error("expected i32Prop to be 42, got " + PropertyTestSingleton.i32Prop);
    }

    // 测试 readonly 属性 - i64
    if (typeof PropertyTestSingleton.i64Prop !== "number") {
        throw new Error("expected i64Prop to be number, got " + typeof PropertyTestSingleton.i64Prop);
    }
    if (PropertyTestSingleton.i64Prop !== 1234567890123) {
        throw new Error("expected i64Prop to be 1234567890123, got " + PropertyTestSingleton.i64Prop);
    }

    // 测试 readonly 属性 - f32
    if (typeof PropertyTestSingleton.f32Prop !== "number") {
        throw new Error("expected f32Prop to be number, got " + typeof PropertyTestSingleton.f32Prop);
    }
    if (Math.abs(PropertyTestSingleton.f32Prop - 3.14) > 0.001) {
        throw new Error("expected f32Prop to be ~3.14, got " + PropertyTestSingleton.f32Prop);
    }

    // 测试 readonly 属性 - f64
    if (typeof PropertyTestSingleton.f64Prop !== "number") {
        throw new Error("expected f64Prop to be number, got " + typeof PropertyTestSingleton.f64Prop);
    }
    if (Math.abs(PropertyTestSingleton.f64Prop - 2.718281828459045) > 0.000001) {
        throw new Error("expected f64Prop to be ~2.718, got " + PropertyTestSingleton.f64Prop);
    }

    // 测试 readonly 属性 - string
    if (typeof PropertyTestSingleton.stringProp !== "string") {
        throw new Error("expected stringProp to be string, got " + typeof PropertyTestSingleton.stringProp);
    }
    if (PropertyTestSingleton.stringProp !== "hello") {
        throw new Error("expected stringProp to be 'hello', got '" + PropertyTestSingleton.stringProp + "'");
    }

    // 测试 read-write 属性 - bool
    if (typeof PropertyTestSingleton.mutableBool !== "boolean") {
        throw new Error("expected mutableBool to be boolean, got " + typeof PropertyTestSingleton.mutableBool);
    }
    if (PropertyTestSingleton.mutableBool !== false) {
        throw new Error("expected mutableBool to be false, got " + PropertyTestSingleton.mutableBool);
    }
    PropertyTestSingleton.mutableBool = true;
    if (PropertyTestSingleton.mutableBool !== true) {
        throw new Error("expected mutableBool to be true after set, got " + PropertyTestSingleton.mutableBool);
    }

    // 测试 read-write 属性 - i32
    if (typeof PropertyTestSingleton.mutableI32 !== "number") {
        throw new Error("expected mutableI32 to be number, got " + typeof PropertyTestSingleton.mutableI32);
    }
    if (PropertyTestSingleton.mutableI32 !== 0) {
        throw new Error("expected mutableI32 to be 0, got " + PropertyTestSingleton.mutableI32);
    }
    PropertyTestSingleton.mutableI32 = 99;
    if (PropertyTestSingleton.mutableI32 !== 99) {
        throw new Error("expected mutableI32 to be 99 after set, got " + PropertyTestSingleton.mutableI32);
    }

    // 测试 read-write 属性 - string
    if (typeof PropertyTestSingleton.mutableString !== "string") {
        throw new Error("expected mutableString to be string, got " + typeof PropertyTestSingleton.mutableString);
    }
    if (PropertyTestSingleton.mutableString !== "") {
        throw new Error("expected mutableString to be '', got '" + PropertyTestSingleton.mutableString + "'");
    }
    PropertyTestSingleton.mutableString = "updated";
    if (PropertyTestSingleton.mutableString !== "updated") {
        throw new Error("expected mutableString to be 'updated', got '" + PropertyTestSingleton.mutableString + "'");
    }

    console.log("PropertyTestSingleton tests passed");
})();