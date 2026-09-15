#ifndef _STUB_MATH_H
#define _STUB_MATH_H
#define INFINITY (__builtin_inff())
#define NAN (__builtin_nanf(""))
#define isnan(x) __builtin_isnan(x)
#define isinf(x) __builtin_isinf(x)
#define isfinite(x) __builtin_isfinite(x)
#define fabs(x) __builtin_fabs(x)
#define floor(x) __builtin_floor(x)
#define ceil(x) __builtin_ceil(x)
#define sqrt(x) __builtin_sqrt(x)
#define trunc(x) __builtin_trunc(x)
#define copysign(x, y) __builtin_copysign(x, y)
double pow(double, double);
double fmod(double, double);
double log2(double);
#endif
