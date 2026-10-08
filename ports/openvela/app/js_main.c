/****************************************************************************
 * ports/openvela/app/js_main.c
 *
 * NSH builtin: run JS scripts with the mquickjs engine on OpenVela/NuttX sim.
 *
 * Usage: js <script.js> [<script2.js> ...]
 *
 * Output protocol (sentinel lines, scanned by the host-side runner — NSH
 * builtins have no scriptable exit code):
 *   CASE <basename> PASS
 *   CASE <basename> FAIL: <message>
 *   CASES <passed>/<total> PASS     (summary line)
 *
 * Integration (see ports/openvela/README.md and the Phase 1 section of
 * docs/planning/2026/2026-10-08-openvela-port.md):
 * - SOURCE-LEVEL integration: the engine's .c files (incl. mqjs_stdlib.c)
 *   are compiled by the openvela apps build with the NuttX toolchain, so
 *   every libc binding (setjmp/longjmp, malloc/free) stays on the NuttX side.
 * - Embedding contract (same as the engine's own mqjs.c REPL): the platform
 *   hooks referenced by the generated stdlib table (print, Date, timers,
 *   load) are defined HERE, BEFORE including mqjs_stdlib.h — the generated
 *   tables reference them at include time.
 * - the JS heap is a caller-provided block: JS_NewContext() only asserts a
 *   fixed minimum and fails SILENTLY if the block is too small (mquickjs.h
 *   "JS_NewContext notes") — hence the explicit size + NULL check below.
 ****************************************************************************/

/****************************************************************************
 * Included Files
 ****************************************************************************/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/time.h>

#include "mquickjs.h"
#include "mquickjs_priv.h"

/****************************************************************************
 * Pre-processor Definitions
 ****************************************************************************/

/* JS heap for the script contexts. 2 MiB is ample for the case corpus on
 * sim; the engine lives entirely inside this block. */

#define JS_HEAP_SIZE (2 * 1024 * 1024)

/****************************************************************************
 * Platform hooks required by the generated stdlib table
 * (implementations mirror deps/mquickjs/mqjs.c)
 *
 * These must appear BEFORE including mqjs_stdlib.h below.
 ****************************************************************************/

static JSValue js_print(JSContext *ctx, JSValue *this_val, int argc,
                        JSValue *argv)
{
    int i;

    for (i = 0; i < argc; i++)
    {
        if (i != 0)
        {
            putchar(' ');
        }

        if (JS_IsString(ctx, argv[i]))
        {
            JSCStringBuf buf;
            const char *str;
            size_t len;
            str = JS_ToCStringLen(ctx, &len, argv[i], &buf);
            fwrite(str, 1, len, stdout);
        }
        else
        {
            JS_PrintValueF(ctx, argv[i], JS_DUMP_LONG);
        }
    }
    putchar('\n');
    return JS_UNDEFINED;
}

static JSValue js_gc(JSContext *ctx, JSValue *this_val, int argc,
                     JSValue *argv)
{
    JS_GC(ctx);
    return JS_UNDEFINED;
}

static int64_t get_time_ms(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + (ts.tv_nsec / 1000000);
}

static int64_t get_date_ms(void)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + (tv.tv_usec / 1000);
}

/* Non-static: referenced by the generated stdlib table. */

JSValue js_date_constructor(JSContext *ctx, JSValue *this_val,
                            int argc, JSValue *argv)
{
    double val;
    argc &= ~FRAME_CF_CTOR;
    if (argc == 0)
    {
        val = get_date_ms();
    }
    else if (argc == 1 && JS_IsNumber(ctx, argv[0]))
    {
        if (JS_ToNumber(ctx, &val, argv[0]))
        {
            return JS_EXCEPTION;
        }
    }
    else
    {
        return JS_ThrowTypeError(ctx, "unsupported Date() parameter");
    }
    return JS_NewDate(ctx, val);
}

static JSValue js_date_now(JSContext *ctx, JSValue *this_val, int argc,
                           JSValue *argv)
{
    return JS_NewInt64(ctx, get_date_ms());
}

static JSValue js_performance_now(JSContext *ctx, JSValue *this_val,
                                  int argc, JSValue *argv)
{
    return JS_NewInt64(ctx, get_time_ms());
}

/* load a script and eval it (hostfs paths work like any NuttX path) */

static JSValue js_load(JSContext *ctx, JSValue *this_val, int argc,
                       JSValue *argv)
{
    const char *filename;
    JSCStringBuf buf_str;
    FILE *f;
    long len;
    char *buf;
    JSValue ret;

    filename = JS_ToCString(ctx, argv[0], &buf_str);
    if (!filename)
    {
        return JS_EXCEPTION;
    }

    f = fopen(filename, "rb");
    if (f == NULL)
    {
        return JS_ThrowTypeError(ctx, "could not load file '%s'", filename);
    }
    if (fseek(f, 0, SEEK_END) != 0 || (len = ftell(f)) < 0)
    {
        fclose(f);
        return JS_ThrowTypeError(ctx, "could not size file '%s'", filename);
    }
    rewind(f);
    buf = malloc((size_t)len + 1);
    if (buf == NULL)
    {
        fclose(f);
        return JS_EXCEPTION;
    }
    if (len > 0 && fread(buf, 1, (size_t)len, f) != (size_t)len)
    {
        free(buf);
        fclose(f);
        return JS_ThrowTypeError(ctx, "could not read file '%s'", filename);
    }
    fclose(f);
    buf[len] = '\0';

    ret = JS_Eval(ctx, buf, (size_t)len, filename, 0);
    free(buf);
    return ret;
}

/* Timers need an event loop this port does not have — explicit stubs so the
 * symbols exist; scripts calling them get a clear TypeError. */

static JSValue js_setTimeout(JSContext *ctx, JSValue *this_val, int argc,
                             JSValue *argv)
{
    return JS_ThrowTypeError(ctx,
                             "setTimeout is not supported on this port");
}

static JSValue js_clearTimeout(JSContext *ctx, JSValue *this_val, int argc,
                               JSValue *argv)
{
    return JS_UNDEFINED;
}

/* The generated stdlib def (weak js_stdlib) — its tables reference the
 * hooks above, hence the include position. */

#include "mqjs_stdlib.h"

/****************************************************************************
 * Case runner
 ****************************************************************************/

static char *read_file(const char *path, size_t *len_out)
{
    FILE *f;
    char *buf;
    long len;

    f = fopen(path, "rb");
    if (f == NULL)
    {
        return NULL;
    }

    if (fseek(f, 0, SEEK_END) != 0 || (len = ftell(f)) < 0)
    {
        fclose(f);
        return NULL;
    }
    rewind(f);

    buf = malloc((size_t)len + 1);
    if (buf == NULL)
    {
        fclose(f);
        return NULL;
    }

    if (len > 0 && fread(buf, 1, (size_t)len, f) != (size_t)len)
    {
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);

    buf[len] = '\0';
    *len_out = (size_t)len;
    return buf;
}

static const char *basename_of(const char *path)
{
    const char *slash = strrchr(path, '/');
    return slash != NULL ? slash + 1 : path;
}

/* Some editors add a UTF-8 BOM; QuickJS doesn't accept it. */

static void strip_bom(char *buf, size_t *len)
{
    if (*len >= 3 && (unsigned char)buf[0] == 0xef
        && (unsigned char)buf[1] == 0xbb && (unsigned char)buf[2] == 0xbf)
    {
        *len -= 3;
        memmove(buf, buf + 3, *len + 1);
    }
}

static int run_case(const char *path)
{
    const char *name = basename_of(path);
    size_t len;
    char *buf;
    void *mem;
    JSContext *ctx;
    JSValue rv;
    int failed;

    buf = read_file(path, &len);
    if (buf == NULL)
    {
        printf("CASE %s FAIL: cannot read file\n", name);
        return 1;
    }
    strip_bom(buf, &len);

    mem = malloc(JS_HEAP_SIZE);
    if (mem == NULL)
    {
        printf("CASE %s FAIL: JS heap alloc failed\n", name);
        free(buf);
        return 1;
    }

    ctx = JS_NewContext(mem, JS_HEAP_SIZE, &js_stdlib);
    if (ctx == NULL)
    {
        printf("CASE %s FAIL: JS_NewContext failed (heap too small?)\n", name);
        free(mem);
        free(buf);
        return 1;
    }

    rv = JS_Eval(ctx, buf, len, path, 0);
    failed = JS_IsException(rv);
    if (!failed)
    {
        printf("CASE %s PASS\n", name);
    }
    else
    {
        /* Minimal error reporting: JS_ToCString covers thrown strings and
         * primitives. Dumping Error OBJECTS (JS_PrintValueF) walks the
         * prototype chain and is not safe in this port yet — print a fixed
         * placeholder instead (the case still reports FAIL, which is what
         * the sentinel protocol needs). */

        JSValue exc = JS_GetException(ctx);
        JSCStringBuf cbuf;
        const char *msg = JS_ToCString(ctx, exc, &cbuf);
        printf("CASE %s FAIL: %s\n", name, msg != NULL ? msg : "<unprintable error>");
    }

    JS_GC(ctx);
    JS_FreeContext(ctx);
    free(mem);
    free(buf);
    return failed ? 1 : 0;
}

/****************************************************************************
 * Public Functions
 ****************************************************************************/

int js_main(int argc, char **argv)
{
    int i;
    int total;
    int passed;

    if (argc < 2)
    {
        printf("usage: js <script.js> [<script2.js> ...]\n");
        return 2;
    }

    total = argc - 1;
    passed = 0;
    for (i = 1; i < argc; i++)
    {
        passed += run_case(argv[i]) == 0 ? 1 : 0;
    }

    printf("CASES %d/%d PASS\n", passed, total);
    return passed == total ? 0 : 1;
}
