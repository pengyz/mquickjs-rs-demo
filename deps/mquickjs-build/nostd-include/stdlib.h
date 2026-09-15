#ifndef _STUB_STDLIB_H
#define _STUB_STDLIB_H
#include <stddef.h>
#define NULL ((void *)0)
void abort(void);
void exit(int);
void *malloc(size_t);
void *calloc(size_t, size_t);
void *realloc(void *, size_t);
void free(void *);
long strtol(const char *, char **, int);
double strtod(const char *, char **);
int atoi(const char *);
#endif
