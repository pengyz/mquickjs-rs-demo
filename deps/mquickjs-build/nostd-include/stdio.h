#ifndef _STUB_STDIO_H
#define _STUB_STDIO_H
#include <stddef.h>
#include <stdarg.h>
typedef struct _stub_FILE FILE;
extern FILE *stderr;
extern FILE *stdout;
int printf(const char *, ...);
int fprintf(FILE *, const char *, ...);
int snprintf(char *, size_t, const char *, ...);
int vsnprintf(char *, size_t, const char *, va_list);
int puts(const char *);
int fputs(const char *, FILE *);
size_t fwrite(const void *, size_t, size_t, FILE *);
int fflush(FILE *);
#endif
