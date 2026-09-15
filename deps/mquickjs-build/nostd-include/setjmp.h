#ifndef _STUB_SETJMP_H
#define _STUB_SETJMP_H
/* mquickjs 不使用 setjmp/longjmp；仅为满足 include。 */
typedef int jmp_buf[1];
int setjmp(jmp_buf);
void longjmp(jmp_buf, int);
#endif
