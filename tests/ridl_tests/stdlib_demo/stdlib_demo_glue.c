#include "mquickjs.h"
#include "stdlib_demo_glue.h"

JSValue js_say_hello(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    JSValue rust_argv[0];
    if (argc != 0) {
        return JS_ThrowTypeError(ctx, "say_hello expects no arguments");
    }
    JSValue rust_result = rust_say_hello(ctx, argc, argv);
    return rust_result;
}