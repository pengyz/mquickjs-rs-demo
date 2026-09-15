#ifndef _STUB_ASSERT_H
#define _STUB_ASSERT_H
#include <stddef.h>
#ifdef NDEBUG
#define assert(x) ((void)0)
#else
void __assert_fail_stub(const char *, const char *, unsigned, const char *);
#define assert(x) ((x) ? (void)0 : __assert_fail_stub(#x, __FILE__, __LINE__, __func__))
#endif
#endif
