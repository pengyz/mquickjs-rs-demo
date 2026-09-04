(function () {
    // 测试 MyService 单例
    if (typeof globalThis.MyService === "undefined") {
        throw new Error("expected globalThis.MyService singleton");
    }

    // 测试 hello 方法
    var result = MyService.hello("World");
    if (result !== "Hello, World!") {
        throw new Error("expected 'Hello, World!', got '" + result + "'");
    }

    // 测试 version 属性
    var version = MyService.version;
    if (version !== "1.0.0") {
        throw new Error("expected '1.0.0', got '" + version + "'");
    }

    console.log("{{project-name}} tests passed");
})();