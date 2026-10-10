(function () {
    // 异常受控端到端：
    // 1. 传非函数 → TypeError（glue 的 JS_IsFunction 校验）
    // 2. 回调体内 throw → invoke 受控返回，ctx 不残留异常、可继续工作
    var b = new Button();

    // 1) 非函数参数
    var threw = false;
    try {
        b.setOnClick("not a function");
    } catch (e) {
        threw = true;
    }
    if (!threw) {
        throw new Error("expected TypeError for non-function callback");
    }

    // 2) 回调体内 throw —— registry 捕获并清除，宿主不崩
    b.setOnClick(function (code) {
        throw new Error("boom from callback");
    });
    b.fire(1); // 受控：打印错误但不崩

    // 3) ctx 仍可用：换一个正常回调继续工作
    var ok = 0;
    var b2 = new Button();
    b2.setOnClick(function (c) {
        ok = ok + c;
    });
    b2.fire(7);
    if (ok !== 7) {
        throw new Error("context broken after throwing callback: ok=" + ok);
    }

    console.log("callback bridge error handling OK");
})();
