#include "mquickjs.h"
#include <sys/time.h>

// Forward declarations for Date support (defined below, referenced by generated header)
JSValue js_date_constructor(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv);
JSValue js_date_now(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv);

// NOTE: mquickjs-build compiles this TU with -include mquickjs_ridl_api.h.
// Do not include mquickjs_ridl_register.h here (it defines file-scope roots for
// the ROM build tool).

// 标准库生成的扩展表（由 mqjs_ridl_stdlib 生成）
// NOTE: include order matters. mqjs_ridl_stdlib.h expects RIDL decls to be expanded
// under the same build-time macro environment as mqjs_stdlib_template.c.
#include "mqjs_ridl_stdlib.h"

/* Date support: platform-specific time functions and constructor.
   Upstream mquickjs puts these in mqjs.c (CLI); we provide them here
   for the library build so that `new Date()` works in embedded contexts. */

static int64_t get_date_ms(void)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + (tv.tv_usec / 1000);
}

JSValue js_date_constructor(JSContext *ctx, JSValue *this_val,
                            int argc, JSValue *argv)
{
    double val;
    argc &= ~FRAME_CF_CTOR;
    if (argc == 0) {
        val = get_date_ms();
    } else if (argc == 1 && JS_IsNumber(ctx, argv[0])) {
        if (JS_ToNumber(ctx, &val, argv[0]))
            return JS_EXCEPTION;
    } else {
        return JS_ThrowTypeError(ctx, "unsupported Date() parameter");
    }
    return JS_NewDate(ctx, val);
}

JSValue js_date_now(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv)
{
    return JS_NewInt64(ctx, get_date_ms());
}