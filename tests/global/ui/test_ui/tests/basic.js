(function () {
    // 测试 UI singleton
    if (typeof globalThis.UI === "undefined") {
        throw new Error("expected globalThis.UI singleton");
    }

    // 测试 createElement
    if (typeof UI.createElement !== "function") {
        throw new Error("expected createElement to be function");
    }
    var divId = UI.createElement("div");
    if (typeof divId !== "number" || divId <= 0) {
        throw new Error("expected createElement to return positive number, got " + divId);
    }

    // 测试 setProperty
    if (typeof UI.setProperty !== "function") {
        throw new Error("expected setProperty to be function");
    }
    UI.setProperty(divId, "class", "container");

    // 测试 getProperty
    if (typeof UI.getProperty !== "function") {
        throw new Error("expected getProperty to be function");
    }
    var classValue = UI.getProperty(divId, "class");
    if (classValue !== "container") {
        throw new Error("expected getProperty to return 'container', got '" + classValue + "'");
    }

    // 测试 appendChild
    if (typeof UI.appendChild !== "function") {
        throw new Error("expected appendChild to be function");
    }
    var spanId = UI.createElement("span");
    UI.appendChild(divId, spanId);

    // 测试 render
    if (typeof UI.render !== "function") {
        throw new Error("expected render to be function");
    }
    UI.render();

    console.log("UI tests passed");
})();