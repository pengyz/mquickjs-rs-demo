/*
 * mquickjs标准库功能的C胶水代码头文件
 * 定义与mquickjs集成的接口
 */

#ifndef STDLIB_GLUE_H
#define STDLIB_GLUE_H

#include "mquickjs.h"

/* 注册标准库函数到全局对象 */
int js_register_stdlib_functions(JSContext *ctx);

/* 注册console单例 */
int js_register_console_singleton(JSContext *ctx);

/* 定时器处理函数，需要在主循环中调用 */
void run_timers(JSContext *ctx);

/* 加载文件的辅助函数 */
uint8_t *js_load_file(JSContext *ctx, const char *filename, int *len);

/* 打印错误的辅助函数 */
void js_dump_error(JSContext *ctx);

#endif /* STDLIB_GLUE_H */