(function () {
    // many-shot 端到端：注册一次，触发多次，JS 侧计数累计。
    if (typeof globalThis.Button === "undefined") {
        throw new Error("expected globalThis.Button class");
    }

    var clicks = 0;
    var lastCode = -1;

    var b = new Button();
    b.setOnClick(function (code) {
        clicks = clicks + 1;
        lastCode = code;
    });

    b.fire(2);
    b.fire(3);
    b.fire(5);

    if (clicks !== 3) {
        throw new Error("many-shot failed: expected 3 invocations, got " + clicks);
    }
    if (lastCode !== 5) {
        throw new Error("last code mismatch: expected 5, got " + lastCode);
    }

    console.log("callback bridge many-shot OK (3 invocations)");
})();
