#include "mquickjs.h"
#include <string.h>

/*
 * require() implementation for RIDL module namespace mode.
 *
 * This file is compiled and linked only when ridl-extensions is enabled.
 * It relies on js_ridl_require_table emitted from the generated
 * mquickjs_ridl_register.c (declared in mquickjs_ridl_api.h).
 */

#include "mquickjs_ridl_api.h"

static int parse_u16_no_ws(const char *s, uint16_t *out) {
    uint32_t v = 0;
    const char *p = s;
    if (!p || !*p)
        return -1;
    while (*p) {
        unsigned char c = (unsigned char)*p;
        if (c < '0' || c > '9')
            return -1;
        v = v * 10u + (uint32_t)(c - '0');
        if (v > 65535u)
            return -1;
        p++;
    }
    *out = (uint16_t)v;
    return 0;
}

static int parse_version_3(const char *s, uint16_t *maj, uint16_t *min, uint16_t *pat) {
    // Accept MAJOR[.MINOR[.PATCH]]
    const char *p = s;
    const char *dot1 = NULL;
    const char *dot2 = NULL;

    for (; *p; p++) {
        unsigned char c = (unsigned char)*p;
        if (c == '.') {
            if (!dot1)
                dot1 = p;
            else if (!dot2)
                dot2 = p;
            else
                return -1;
            continue;
        }
        if (c <= ' ')
            return -1;
    }

    if (!dot1) {
        if (parse_u16_no_ws(s, maj))
            return -1;
        *min = 0;
        *pat = 0;
        return 0;
    }

    // major
    {
        char buf[16];
        size_t n = (size_t)(dot1 - s);
        if (n == 0 || n >= sizeof(buf))
            return -1;
        memcpy(buf, s, n);
        buf[n] = '\0';
        if (parse_u16_no_ws(buf, maj))
            return -1;
    }

    if (!dot2) {
        char buf[16];
        size_t n = strlen(dot1 + 1);
        if (n == 0 || n >= sizeof(buf))
            return -1;
        memcpy(buf, dot1 + 1, n);
        buf[n] = '\0';
        if (parse_u16_no_ws(buf, min))
            return -1;
        *pat = 0;
        return 0;
    }

    // minor
    {
        char buf[16];
        size_t n = (size_t)(dot2 - (dot1 + 1));
        if (n == 0 || n >= sizeof(buf))
            return -1;
        memcpy(buf, dot1 + 1, n);
        buf[n] = '\0';
        if (parse_u16_no_ws(buf, min))
            return -1;
    }

    // patch
    {
        char buf[16];
        size_t n = strlen(dot2 + 1);
        if (n == 0 || n >= sizeof(buf))
            return -1;
        memcpy(buf, dot2 + 1, n);
        buf[n] = '\0';
        if (parse_u16_no_ws(buf, pat))
            return -1;
    }

    return 0;
}

static int version_cmp(uint16_t a0, uint16_t a1, uint16_t a2, uint16_t b0, uint16_t b1, uint16_t b2) {
    if (a0 != b0)
        return (a0 < b0) ? -1 : 1;
    if (a1 != b1)
        return (a1 < b1) ? -1 : 1;
    if (a2 != b2)
        return (a2 < b2) ? -1 : 1;
    return 0;
}

static JSValue require_not_found(JSContext *ctx, const char *spec) {
    JS_ThrowTypeError(ctx, "require %s failed: module not found.", spec);
    return JS_EXCEPTION;
}

static int no_ws(const char *s) {
    const char *p = s;
    while (*p) {
        if ((unsigned char)*p <= ' ')
            return 0;
        p++;
    }
    return 1;
}

static int select_match(const RidlRequireEntry *e, int op, uint16_t maj, uint16_t min, uint16_t pat) {
    // op: 0 exact, 1 >, 2 >=, 3 <, 4 <=
    int c = version_cmp(e->v_major, e->v_minor, e->v_patch, maj, min, pat);
    switch (op) {
    case 0:
        return c == 0;
    case 1:
        return c > 0;
    case 2:
        return c >= 0;
    case 3:
        return c < 0;
    case 4:
        return c <= 0;
    default:
        return 0;
    }
}

static JSValue js_global_require(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv) {
    const char *spec;
    const char *at;
    const char *tail;

    (void)this_val;

    if (argc < 1)
        return JS_ThrowTypeError(ctx, "require expects a string");

    JSCStringBuf sbuf;
    spec = JS_ToCString(ctx, argv[0], &sbuf);
    if (!spec)
        return JS_EXCEPTION;

    if (!no_ws(spec)) {
        return require_not_found(ctx, "<invalid>");
    }

    at = strchr(spec, '@');

    const RidlRequireEntry *best = NULL;

    if (!at) {
        // Latest: base only
        const char *base = spec;
        for (int i = 0; i < js_ridl_require_table_len; i++) {
            const RidlRequireEntry *e = &js_ridl_require_table[i];
            if (strcmp(e->module_base, base) != 0)
                continue;
            if (!best || version_cmp(e->v_major, e->v_minor, e->v_patch, best->v_major, best->v_minor, best->v_patch) > 0)
                best = e;
        }
        if (!best) {
            return require_not_found(ctx, base);
        }
    } else {
        // base@...
        size_t base_len = (size_t)(at - spec);
        if (base_len == 0) {
            return require_not_found(ctx, spec);
        }

        char base_buf[256];
        if (base_len >= sizeof(base_buf)) {
            return require_not_found(ctx, spec);
        }
        memcpy(base_buf, spec, base_len);
        base_buf[base_len] = '\0';

        tail = at + 1;
        if (!*tail) {
            return require_not_found(ctx, spec);
        }

        int op = 0;
        if (tail[0] == '>' && tail[1] == '=') {
            op = 2;
            tail += 2;
        } else if (tail[0] == '<' && tail[1] == '=') {
            op = 4;
            tail += 2;
        } else if (tail[0] == '>') {
            op = 1;
            tail += 1;
        } else if (tail[0] == '<') {
            op = 3;
            tail += 1;
        } else {
            op = 0;
        }

        uint16_t maj, min, pat;
        if (parse_version_3(tail, &maj, &min, &pat)) {
            return require_not_found(ctx, spec);
        }

        for (int i = 0; i < js_ridl_require_table_len; i++) {
            const RidlRequireEntry *e = &js_ridl_require_table[i];
            if (strcmp(e->module_base, base_buf) != 0)
                continue;
            if (!select_match(e, op, maj, min, pat))
                continue;
            if (!best || version_cmp(e->v_major, e->v_minor, e->v_patch, best->v_major, best->v_minor, best->v_patch) > 0)
                best = e;
        }

        if (!best) {
            return require_not_found(ctx, spec);
        }
    }

    // Create a new module instance each call.
    JSValue obj = JS_NewObjectClassUser(ctx, best->module_class_id);
    if (JS_IsException(obj)) {
        return obj;
    }

    /*
     * RIDL module exports include ROMClass entries (JS_DEF_CLASS). They need to be
     * materialized into ctor functions on the returned module instance so userland
     * can do: new require("m").MyClass().
     *
     * Scope: instance + one-level prototype (write back as own property).
     */
    {
        if (JS_MaterializeModuleClassExports(ctx, obj) < 0)
            return JS_EXCEPTION;
    }

    return obj;
}

// Used by mqjs_stdlib_template.c.
JSValue js_ridl_require(JSContext *ctx, JSValue *this_val, int argc, JSValue *argv) {
    return js_global_require(ctx, this_val, argc, argv);
}
