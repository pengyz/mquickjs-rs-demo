#ifndef _STUB_SYS_TIME_H
#define _STUB_SYS_TIME_H
#include <stdint.h>

/* 裸机无 POSIX 时钟。mqjs_stdlib_impl.c 的 Date 支持用 gettimeofday 取墙钟，
   交叉编译时仅需声明；实现应由平台提供（或改由嵌入式时基注入）。
   移植注意项：这是核心之外的一个**平台钩子**，见
   docs/knowledge/assessment_core_nostd_port_cost.md */
struct timeval {
    long tv_sec;
    long tv_usec;
};

int gettimeofday(struct timeval *tv, void *tz);

#endif
