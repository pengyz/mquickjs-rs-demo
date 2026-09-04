(function () {
    // 测试 ShapeService singleton（接口类型验证 + singleton 方法测试）
    if (typeof globalThis.ShapeService === "undefined") {
        throw new Error("expected globalThis.ShapeService singleton");
    }

    // 测试 createCircle 方法
    if (typeof ShapeService.createCircle !== "function") {
        throw new Error("expected createCircle to be function");
    }
    var circleArea = ShapeService.createCircle(5);
    if (circleArea !== 25) {
        throw new Error("expected createCircle(5) to be 25, got " + circleArea);
    }

    // 测试 createRectangle 方法
    if (typeof ShapeService.createRectangle !== "function") {
        throw new Error("expected createRectangle to be function");
    }
    var rectArea = ShapeService.createRectangle(3, 4);
    if (rectArea !== 12) {
        throw new Error("expected createRectangle(3, 4) to be 12, got " + rectArea);
    }

    // 测试 describeShape 方法（字符串参数+返回值）
    if (typeof ShapeService.describeShape !== "function") {
        throw new Error("expected describeShape to be function");
    }
    var desc = ShapeService.describeShape("circle");
    if (desc !== "Shape: circle") {
        throw new Error("expected describeShape('circle') to be 'Shape: circle', got '" + desc + "'");
    }

    console.log("Interface tests passed");
})();