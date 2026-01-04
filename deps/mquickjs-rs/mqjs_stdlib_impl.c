/*
 * mquickjs standard library implementation
 *
 * This file contains implementations of standard library functions that were
 * originally in mqjs.c but are needed when building mquickjs as a library.
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

/* Forward declaration of helper functions */
uint8_t *js_load_file(JSContext *ctx, const char *filename, int *len);

/* Print function implementation */
static JSValue js_print(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv)
{
    int i;
    JSValue v;
    
    for(i = 0; i < argc; i++) {
        if (i != 0)
            putchar(' ');
        v = argv[i];
        if (JS_IsString(ctx, v)) {
            JSCStringBuf buf;
            const char *str;
            size_t len;
            str = JS_ToCStringLen(ctx, &len, v, &buf);
            fwrite(str, 1, len, stdout);
        } else {
            JS_PrintValueF(ctx, argv[i], JS_DUMP_LONG);
        }
    }
    putchar('\n');
    return JS_UNDEFINED;
}

/* Garbage collection function */
JSValue js_gc(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    JS_GC(ctx);
    return JS_UNDEFINED;
}

#if defined(__linux__) || defined(__APPLE__)
static int64_t get_time_ms(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000 + (ts.tv_nsec / 1000000);
}
#else
static int64_t get_time_ms(void)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + (tv.tv_usec / 1000);
}
#endif

/* Date.now implementation */
JSValue js_date_now(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return JS_NewFloat64(ctx, (int64_t)tv.tv_sec * 1000 + (tv.tv_usec / 1000));
}

/* Performance.now implementation */
JSValue js_performance_now(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    return JS_NewFloat64(ctx, get_time_ms());
}

static uint8_t *load_file(const char *filename, int *plen)
{
    FILE *f;
    uint8_t *buf;
    int buf_len;

    f = fopen(filename, "rb");
    if (!f) {
        perror(filename);
        exit(1);
    }
    fseek(f, 0, SEEK_END);
    buf_len = ftell(f);
    fseek(f, 0, SEEK_SET);
    buf = malloc(buf_len + 1);
    fread(buf, 1, buf_len, f);
    buf[buf_len] = '\0';
    fclose(f);
    if (plen)
        *plen = buf_len;
    return buf;
}

/* Load a script function */
static JSValue js_load(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv)
{
    const char *filename;
    JSCStringBuf buf_str;
    uint8_t *buf;
    int buf_len;
    JSValue ret;
    
    filename = JS_ToCString(ctx, argv[0], &buf_str);
    if (!filename)
        return JS_EXCEPTION;
    buf = load_file(filename, &buf_len);

    ret = JS_Eval(ctx, (const char *)buf, buf_len, filename, 0);
    free(buf);
    return ret;
}

/* Timer implementation */
typedef struct {
    int allocated;  // Use int instead of BOOL
    JSValue func;
    int64_t timeout; /* in ms */
    JSContext *ctx;
} JSTimer;

#define MAX_TIMERS 16

static JSTimer js_timer_list[MAX_TIMERS];

JSValue js_setTimeout(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    JSTimer *th;
    int delay, i;
    
    if (!JS_IsFunction(ctx, argv[0]))
        return JS_ThrowTypeError(ctx, "not a function");
    if (JS_ToInt32(ctx, &delay, argv[1]))
        return JS_EXCEPTION;
    for(i = 0; i < MAX_TIMERS; i++) {
        th = &js_timer_list[i];
        if (!th->allocated) {
            th->func = argv[0];  // No JS_DupValue in mquickjs, just copy
            th->timeout = get_time_ms() + delay;
            th->allocated = 1;  // Use 1 instead of TRUE
            th->ctx = ctx;
            return JS_NewInt32(ctx, i);
        }
    }
    return JS_ThrowInternalError(ctx, "too many timers");
}

JSValue js_clearTimeout(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)
{
    int timer_id;
    JSTimer *th;

    if (JS_ToInt32(ctx, &timer_id, argv[0]))
        return JS_EXCEPTION;
    if (timer_id >= 0 && timer_id < MAX_TIMERS) {
        th = &js_timer_list[timer_id];
        if (th->allocated) {
            // No JS_FreeValue in mquickjs, just clear
            th->allocated = 0;  // Use 0 instead of FALSE
        }
    }
    return JS_UNDEFINED;
}

void run_timers(JSContext *ctx)
{
    int64_t min_delay, delay, cur_time;
    int has_timer;  // Use int instead of BOOL
    int i;
    JSTimer *th;
    struct timespec ts;

    for(;;) {
        min_delay = 1000;
        cur_time = get_time_ms();
        has_timer = 0;  // Use 0 instead of FALSE
        for(i = 0; i < MAX_TIMERS; i++) {
            th = &js_timer_list[i];
            if (th->allocated) {
                has_timer = 1;  // Use 1 instead of TRUE
                delay = th->timeout - cur_time;
                if (delay <= 0) {
                    JSValue ret;
                    /* the timer expired */
                    if (JS_StackCheck(ctx, 2))
                        goto fail;
                    
                    JS_PushArg(ctx, th->func);  // Push function
                    JS_PushArg(ctx, JS_NULL);   // Push this
                    ret = JS_Call(ctx, 0);      // Call with 0 arguments
                    if (JS_IsException(ret)) {
                    fail:
                        // Error handling would go here
                        break;
                    }
                    th->allocated = 0;  // Use 0 instead of FALSE
                    min_delay = 0;
                    break;
                } else if (delay < min_delay) {
                    min_delay = delay;
                }
            }
        }
        if (!has_timer)
            break;
        if (min_delay > 0) {
            ts.tv_sec = min_delay / 1000;
            ts.tv_nsec = (min_delay % 1000) * 1000000;
            nanosleep(&ts, NULL);
        }
    }
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

#include "mqjs_stdlib.h"  // 包含标准库头文件
