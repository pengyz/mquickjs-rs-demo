/*
 * RIDL Standard Library Builder
 * 
 * 用于生成仅包含RIDL定义函数的头文件
 * 此工具将从RIDL模块定义中生成标准库头文件
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* 为构建工具定义，包含mquickjs.h以获取类型定义 */
#ifdef __HOST__
#include "mquickjs.h"
#else
#include "mquickjs_build.h"
#endif

/* Include RIDL-generated standard library extensions */
#include "mquickjs_ridl_register.h"

int main(int argc, char **argv)
{
    // 空的全局对象定义，因为我们只关心RIDL扩展
    static const JSCFunctionListEntry empty_global_obj[] = {
        JS_PROP_END,
    };
    
    return build_atoms("js_stdlib_demo", empty_global_obj, NULL, argc, argv);
}