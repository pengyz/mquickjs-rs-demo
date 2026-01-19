#include "mquickjs.h"

// NOTE: mquickjs-build compiles this TU with -include mquickjs_ridl_api.h.
// Do not include mquickjs_ridl_register.h here (it defines file-scope roots for
// the ROM build tool).

// 标准库生成的扩展表（由 mqjs_ridl_stdlib 生成）
// NOTE: include order matters. mqjs_ridl_stdlib.h expects RIDL decls to be expanded
// under the same build-time macro environment as mqjs_stdlib_template.c.
#include "mqjs_ridl_stdlib.h"