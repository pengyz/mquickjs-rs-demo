/*
 * RIDL Standard Library Builder
 * 
 * 用于生成仅包含RIDL定义函数的头文件
 * 此工具将从RIDL模块定义中生成标准库头文件
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "mquickjs_build.h"

// 从RIDL生成的胶水代码中引入函数声明
extern JSValue js_say_hello(JSContext *ctx, JSValue this_val, int argc, JSValue *argv);

static const JSCFunctionListEntry js_say_hello_funcs[] = {
    JS_CFUNC_DEF("say_hello", 0, js_say_hello),
    JS_PROP_END,
};

static const JSCFunctionListEntry js_global_obj[] = {
    JS_OBJECT_DEF("stdlib_demo", js_say_hello_funcs),
    JS_PROP_END,
};

int main(int argc, char **argv)
{
    return build_atoms("js_stdlib", js_global_obj, NULL, argc, argv);
}