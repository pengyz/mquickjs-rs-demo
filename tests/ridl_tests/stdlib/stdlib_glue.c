/*
 * mquickjs标准库功能的C胶水代码实现
 * 实现stdlib.ridl中定义的平台相关功能
 * 此文件提供注册到标准库静态表中的C函数
 * 将JSValue转换为Rust数据类型，然后调用Rust实现
 */

#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdarg.h>
#include <inttypes.h>
#include <string.h>
#include <assert.h>
#include <ctype.h>
#include <time.h>
#include <sys/time.h>
#include <math.h>
#include <fcntl.h>
#include <unistd.h>

#include "mquickjs.h"

/* Forward declaration of Rust implementations */
JSValue rust_console_log(JSContext *ctx, JSValue this_val, int argc, char **args);
JSValue rust_console_error(JSContext *ctx, JSValue this_val, int argc, char **args);
double rust_date_now(JSContext *ctx);
double rust_performance_now(JSContext *ctx);
JSValue rust_gc(JSContext *ctx);
JSValue rust_load(JSContext *ctx, const char *filename);
JSValue rust_setTimeout(JSContext *ctx, JSValue func, int delay);
JSValue rust_clearTimeout(JSContext *ctx, int timer_id);
JSValue rust_say_hello(JSContext *ctx);

/* Helper function to convert JSValue to C string */
char* js_to_cstring(JSContext *ctx, JSValue val) {
    JSCStringBuf buf;
    const char *str = JS_ToCString(ctx, val, &buf);
    if (!str) {
        return NULL;
    }
    
    size_t len = strlen(str);
    char *result = malloc(len + 1);
    if (result) {
        strcpy(result, str);
    }
    // Note: mquickjs does not have JS_FreeCString function
    // The string returned by JS_ToCString is managed internally
    return result;
}

/* Helper function to convert JSValue to int */
int js_to_int(JSContext *ctx, JSValue val, int *result) {
    return JS_ToInt32(ctx, result, val);
}

/* say_hello实现 - 调用Rust实现 */
JSValue js_say_hello(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    return JS_NewString(ctx, "hello world");
}

/* console.log实现 - 调用Rust实现 */
JSValue js_console_log(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    if (argc == 0) {
        return JS_UNDEFINED;
    }
    
    // 将JSValue参数转换为C字符串数组
    char **args = malloc(argc * sizeof(char*));
    if (!args) {
        return JS_UNDEFINED;
    }
    
    for (int i = 0; i < argc; i++) {
        args[i] = js_to_cstring(ctx, argv[i]);
    }
    
    JSValue result = rust_console_log(ctx, this_val, argc, args);
    
    // 清理分配的字符串
    for (int i = 0; i < argc; i++) {
        if (args[i]) {
            free(args[i]);
        }
    }
    free(args);
    
    return result;
}

/* console.error实现 - 调用Rust实现 */
JSValue js_console_error(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    if (argc == 0) {
        return JS_UNDEFINED;
    }
    
    // 将JSValue参数转换为C字符串数组
    char **args = malloc(argc * sizeof(char*));
    if (!args) {
        return JS_UNDEFINED;
    }
    
    for (int i = 0; i < argc; i++) {
        args[i] = js_to_cstring(ctx, argv[i]);
    }
    
    JSValue result = rust_console_error(ctx, this_val, argc, args);
    
    // 清理分配的字符串
    for (int i = 0; i < argc; i++) {
        if (args[i]) {
            free(args[i]);
        }
    }
    free(args);
    
    return result;
}

/* Date.now实现 - 调用Rust实现 */
JSValue js_date_now(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    double result = rust_date_now(ctx);
    return JS_NewFloat64(ctx, result);
}

/* Performance.now实现 - 调用Rust实现 */
JSValue js_performance_now(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    double result = rust_performance_now(ctx);
    return JS_NewFloat64(ctx, result);
}

/* gc实现 - 调用Rust实现 */
JSValue js_gc(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    return rust_gc(ctx);
}

/* load实现 - 调用Rust实现 */
JSValue js_load(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    if (argc < 1) {
        return JS_UNDEFINED;
    }
    
    char *filename = js_to_cstring(ctx, argv[0]);
    if (!filename) {
        return JS_UNDEFINED;
    }
    
    JSValue result = rust_load(ctx, filename);
    free(filename);
    
    return result;
}

/* setTimeout实现 - 调用Rust实现 */
JSValue js_setTimeout(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    if (argc < 2) {
        return JS_UNDEFINED;
    }
    
    JSValue func = argv[0];
    int delay;
    if (js_to_int(ctx, argv[1], &delay) != 0) {
        return JS_EXCEPTION;
    }
    
    return rust_setTimeout(ctx, func, delay);
}

/* clearTimeout实现 - 调用Rust实现 */
JSValue js_clearTimeout(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    if (argc < 1) {
        return JS_UNDEFINED;
    }
    
    int timer_id;
    if (js_to_int(ctx, argv[0], &timer_id) != 0) {
        return JS_EXCEPTION;
    }
    
    return rust_clearTimeout(ctx, timer_id);
}

/* Helper function to load files */
uint8_t *js_load_file(JSContext *ctx, const char *filename, int *len)
{
    FILE *f;
    uint8_t *buf;
    int buf_len;

    f = fopen(filename, "rb");
    if (!f) {
        // Note: JS_ThrowReferenceError is not in mquickjs.h, using printf instead
        fprintf(stderr, "Could not load file '%s'\n", filename);
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    buf_len = ftell(f);
    fseek(f, 0, SEEK_SET);
    buf = malloc(buf_len + 1);  // Use malloc instead of js_malloc
    if (!buf) {
        fclose(f);
        fprintf(stderr, "Out of memory\n");
        return NULL;
    }
    fread(buf, 1, buf_len, f);
    buf[buf_len] = '\0';
    fclose(f);
    if (len)
        *len = buf_len;
    return buf;
}

/* Helper function to print errors */
void js_dump_error(JSContext *ctx)
{
    JSValue exception_val;
    int is_error;
    
    exception_val = JS_GetException(ctx);
    is_error = JS_IsError(ctx, exception_val);
    JS_PrintValueF(ctx, exception_val, JS_DUMP_LONG);
    if (is_error) {
        JSValue stack_val = JS_GetPropertyStr(ctx, exception_val, "stack");
        if (!JS_IsUndefined(stack_val)) {
            printf("\n");
            JS_PrintValueF(ctx, stack_val, JS_DUMP_LONG);
        }
        // No JS_FreeValue in mquickjs
    }
    // No JS_FreeValue in mquickjs
}