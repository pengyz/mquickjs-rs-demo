<!-- planning-meta
status: 未复核
tags: build, context-init, engine, require, ridl
replaced_by:
- docs/ridl/overview.md
- docs/ridl/context-init.md
- docs/ridl/require-materialize.md
- docs/build/pipeline.md
-->

> 状态：**未复核**（`build` `context-init` `engine` `require` `ridl`）
>
> 现行口径/替代：
> docs/ridl/overview.md
> docs/ridl/context-init.md
> docs/ridl/require-materialize.md
> docs/build/pipeline.md
>
> 关键结论：
> - （待补充：3~5 条）
# RIDL module 命名空间（module ns）+ require() 机制（V1 实现方案）

状态：草案（待评审确认后进入实现）

> 目标：实现 RIDL 语法层面的 `module <name>` 命名空间模式：
> - RIDL 文件声明 `module foo.bar` 后，该文件内导出不再进入 global，而是被收拢在一个“模块实例对象”上；
> - JS 侧通过 `require("foo.bar")` 获取该模块实例对象；
> - `require()` **每次返回新实例**；模块实例对象 **不可 new**（不暴露 constructor）。
>
> 约束：mquickjs 的注册/ROM 表必须在编译期静态生成，不允许运行时动态注册 QuickJS C API。

---

## 0. 背景与已确认语义（来自本次讨论）

### 0.1 module name + version（强约束）
- module 名称 **必须包含版本号**：`<base>@<version>`，例如：`system.network@1.2`。
- 不允许空格：`require()` 入参 spec 以及 RIDL `module ...` 声明中，`@` 前后均不得包含空白字符。

### 0.2 module 内允许的 RIDL item
- **允许**：除 `singleton` 外的所有 RIDL item。
  - `fn`：作为模块实例方法导出（`require('m').foo()`）。
  - `class`：作为模块实例的导出成员（`new require('m').Connect()`）。
  - `interface`/`using`/`enum`/`type` 等：按现有生成器语义处理；**V1 不导出到 JS**（仅作为类型系统输入）。
- **禁止**：`singleton` 出现在 module 中。
  - 原因：singleton 主要用于 `console` 这类“global 上的内置对象支持”；module 本身已是实例对象，singleton 语义重复且容易混淆。

### 0.3 require() 入参
- `require(spec: string)`，其中 `spec` 使用 **原始字符串形式**（例如：`"system.network@1.2"` 或 `"system.network@>1.2"`）。
- `normalize` 仅用于 **生成 C 符号名 / class id 符号名**；JS 侧匹配与选择只基于原始字符串解析结果。

### 0.4 require() 语义
- `require()` 每次调用创建并返回一个新的模块实例对象（不缓存）。
- 若用户希望单例语义，应使用 `singleton`（与 module 模式无关）。

---

## 1. 技术可行性调研结论（mquickjs-build / ROM）

### 1.1 “无 constructor 的 native class”可行
- mquickjs 提供 `JS_NewObjectClassUser(ctx, class_id)`，允许 C 侧直接创建某 class_id 的实例对象。
- 因此可以做到：
  - JS 侧不暴露 module class 的 constructor（用户无法 `new`）；
  - 但 `require()` 内部仍可通过 `JS_NewObjectClassUser()` 创建实例。

### 1.2 ROM/build 工具对“非 global 对象挂 class_def 属性”的支持情况
结论：**支持**（以 ROM 表角度可表达）。

依据：`deps/mquickjs/mquickjs_build.c` 的 `define_props()`/`define_value()` 实现：
- `JS_DEF_CLASS` 作为一种 property value 类型存在；
- `define_atoms_props()`/`define_value()` 对 `JS_DEF_CLASS` 的处理并不要求 props 只能出现在 global 对象；
- 限制点仅在 `JS_DEF_CGETSET`：若 `is_global_object` 则禁止 getter/setter，但这不影响 `JS_DEF_CLASS`。

意味着：
- 我们可以为“模块实例对象”定义一个 `JSClassDef`，其 `class_props`（对象属性表）可包含 `JS_DEF_CLASS`，从而把导出的 RIDL class（例如 Connect）作为属性挂到模块实例对象上；
- 或者在 `require()` 里对新创建的实例对象调用一个“apply props”函数（如果 mquickjs 已提供类似工具）。

> 注：上述结论关注的是 ROM 表与 props 表达能力；实现细节可在“module class_def 的 class_props / proto_props”中落地。

---

## 2. 产物与数据结构设计（新增）

### 2.1 require table（C 侧静态表）
由 ridl-tool（聚合阶段）生成 C 侧静态表，用于支持版本选择：

```c
typedef struct {
    const char *module_base;    // 例如 "system.network"
    uint16_t v_major;
    uint16_t v_minor;
    uint16_t v_patch;
    int module_class_id;        // JS_CLASS_<normalized MODULE_NAME>
} RidlRequireEntry;

extern const RidlRequireEntry js_ridl_require_table[];
extern const int js_ridl_require_table_len;
```

说明：
- `module_base` 用于 require() base 匹配；
- 版本号以数值形式存放，运行时做三元组比较；
- `module_class_id` 直接用于 `JS_NewObjectClassUser()` 创建 module 实例。

备注：
- 若后续需要更快查找，可引入按 `module_base` 的 hash/二分索引（V1 不做）。

### 2.2 module class（每个 module 一个 class）
- 每个 `module <name>` 对应一个“module class”。
- module class 的职责（V1 冻结为 **object class**，而非传统 `class + prototype` 模型）：
  - **object props**：module 内 `fn` 生成的可调用导出（`JS_DEF_CFUNC` 等），作为模块实例对象的属性函数。
  - **object props**：module 内导出的 `class` 构造器属性（`JS_DEF_CLASS`），使其可通过 `require('m').Connect` 访问。
- **不导出 constructor 到 JS（强约束）**：
  - module class_def **必须**设置 `func_name = NULL`，确保 ROM/build 不生成可见的 ctor/prototype 体系，避免用户通过 `m.constructor` 等路径拿到可调用 constructor。
  - `require()` 负责创建实例：`JS_NewObjectClassUser(ctx, <module-class-id>)`。

### 2.3 稳定 class id 的对齐（V1）
- V1 以 ROM/build 生成的 `JS_CLASS_*` 编译期常量作为 class id 的唯一来源。
- module class 作为 RIDL user class 的一种，应被 ROM/build 分配为 `JS_CLASS_USER + i`，并在生成的 `mquickjs_ridl_register.h` 中以 `#define JS_CLASS_<...> (JS_CLASS_USER + i)` 形式出现。

> 说明：本仓库当前没有可用的 `-c` 导出命令链路，因此 V1 不使用 `RIDL_CLASS_*`。

---

## 3. C 侧实现：require() 注入与行为

### 3.1 require() 函数实现位置
- 优先在 C 侧实现（减少 Rust FFI 暴露面）。
- 注入点：stdlib 模板（例如 `template.c` 或等价入口），并作为 global function 注入。

### 3.2 require() 输入 spec 的语法（V1）

设 module 的声明形式为：`<base>@<version>`，例如 `system.network@1.2`。

`require(spec)` 中 `spec` 允许三类：

1) **不带版本**：`<base>`
- 例：`require("system.network")`
- 规则：选择该 base 下 **最高版本**。

2) **精确版本**：`<base>@<version>`
- 例：`require("system.network@1.2")`
- 规则：必须精确匹配该版本（内部会把 `1`/`1.2` 规范化为三段比较）。

3) **版本约束（一元）**：`<base>@<op><version>`
- 例：`require("system.network@>1.2")` / `@>=1.2` / `@<1.2` / `@<=1.2`
- 规则：过滤该 base 下满足约束的版本集合，并取其中 **最高版本**。

强约束：
- 不允许空格（`@` 前后、`op` 周围、版本号中均不允许出现空白）。
- module 声明 **必须**包含版本号（不允许 `module system.network;`）。

### 3.3 require() 失败的错误模型
- 找不到模块（含：base 不存在、精确版本不存在、约束过滤后为空）：
  - 抛出 TypeError，消息：`require <spec> failed: module not found.`

### 3.4 require table（为版本选择服务）
V1 require-table entry 直接携带：

```c
typedef struct {
    const char *module_base;    // 例如 "system.network"
    uint16_t v_major;
    uint16_t v_minor;
    uint16_t v_patch;
    int module_class_id;        // JS_CLASS_<normalized MODULE_NAME>
} RidlRequireEntry;
```

说明：
- 运行时选择完全依赖 base + (major,minor,patch) 的数值比较；
- `normalized MODULE_NAME` 建议取 full module name（含版本）进行 normalize，避免同 base 不同版本的符号冲突。

### 3.5 require() 伪代码（含版本选择）

```c
static JSValue js_global_require(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    if (argc < 1 || !JS_IsString(argv[0])) {
        return JS_ThrowTypeError(ctx, "require(spec: string)");
    }

    const char *spec = JS_ToCString(ctx, argv[0]);
    if (!spec) return JS_EXCEPTION;

    // parse spec -> base + mode + (op?) + version?
    // - reject any whitespace
    // - version: MAJOR[.MINOR[.PATCH]]

    const RidlRequireEntry *best = NULL;
    for (int i = 0; i < js_ridl_require_table_len; i++) {
        const RidlRequireEntry *e = &js_ridl_require_table[i];
        // match base
        // apply mode (none / exact / constraint)
        // choose highest satisfying (major,minor,patch)
    }

    if (!best) {
        JS_FreeCString(ctx, spec);
        return JS_ThrowTypeError(ctx, "require %s failed: module not found.", spec);
    }

    JSValue obj = JS_NewObjectClassUser(ctx, best->module_class_id);
    JS_FreeCString(ctx, spec);
    if (JS_IsException(obj)) return obj;
    return obj;
}
```

---

## 4. 生成器（ridl-tool）改动点（概述）

### 4.1 AST/模型
- 现有模型已经有 `module_name` / module_ns 逻辑；本方案新增“module object（module class）”这一生成目标。

### 4.2 代码生成（C 侧）
扩展 `mquickjs_ridl_register_h.rs.j2`：
- 增加 `require-table` 的 decl/def；
- 增加全局 `require` 的 `JS_CFUNC_DEF("require", 1, js_global_require)` 注入；
- 为每个 module 生成 module class_def（proto_props + class_props）。

### 4.3 校验与限制
- module 内禁止 singleton：解析/resolve 阶段报错。
- module 内导出名冲突（同名 fn/class）：生成器报错（避免运行时静默覆盖）。
- module name 字符集限制：至少保证可作为 C 字符串常量，并且 `normalize` 结果可用于 C 符号。

---

## 5. 测试矩阵（V1）

### 5.1 新增一个 module 用例 crate
在 `tests/` 下新增一个最小 module 用例（或复用现有 tests 结构新增一个 crate）：

```ridl
module system.network.request;

class Connect {
    fn ping() -> int;
}

fn add(a: int, b: int) -> int;
```

JS 用例：
- `var m = require("system.network.request");`
- `assert(typeof m.add === 'function')`
- `assert(m.add(1,2) === 3)`
- `assert(typeof m.Connect === 'function')`（构造器函数）
- `var c = new m.Connect(); assert(c.ping() === 1)`（示例）
- `var m2 = require("system.network.request"); assert(m !== m2)`（验证“每次新实例”）

### 5.2 错误用例
- `require()` 无参数 / 非 string：TypeError
- `require("not-exist")`：TypeError("module not found")（错误类型可在实现前再确认一次）
- module 内如果出现 singleton：生成阶段失败（工具报错）

---

## 6. 仍需确认的少量细节（实现前冻结）

1) `require("xxx")` 找不到模块时：错误类型是否固定为 `TypeError`？消息文本是否需要规范化？
2) module class 的 class_id 宏命名规则（`RIDL_CLASS_<...>__MODULE` 的 `<...>` 具体 normalize 规则）。
3) module 内 `interface` / `type` 是否需要作为 JS 属性导出？（V1 建议不导出，纯类型系统输入）

---

## 7. Phase0 验证：JS_CLASS_* 数值语义与 JS_NewObjectClassUser

### 7.1 验证目的
本方案要求：require() / glue 在创建 user class 实例时可直接调用：
- `JS_NewObjectClassUser(ctx, JS_CLASS_*)`

因此必须验证 `JS_CLASS_*` 的数值语义确实等价于“JS class id（从 JS_CLASS_USER 起分配的连续整数）”，而不是其他概念（例如 ROM 表内部 offset）。

### 7.2 验证动作（已执行）
- 将 `tests/global/class/test_class` 加入 app dependencies（root `Cargo.toml` 新增 `ridl_test_g_class`）。
- 执行：`cargo run -p ridl-builder -- prepare --profile framework`
- 观察生成物：
  - `target/mquickjs-build/.../debug/ridl/include/mquickjs_ridl_register.h`

### 7.3 关键观察结论
- class 用例触发后，ROM 输入头文件中出现：
  - `#define JS_CLASS_GLOBAL_USER (JS_CLASS_USER + 0)`
  - `#define JS_CLASS_COUNT (JS_CLASS_USER + 1)`
  - `static const JSClassDef js_global_class_user_class_def = JS_CLASS_DEF(..., JS_CLASS_GLOBAL_USER, ...)`

这表明：
- **user class id 在 ROM 侧的权威形式就是 `JS_CLASS_USER + i` 的编译期常量**；
- `JS_NewObjectClassUser(ctx, JS_CLASS_GLOBAL_USER)` 的入参语义明确为“JS class id”。

> 说明：本仓库当前没有可用的 `-c` 导出命令链路，因此 Phase0 直接通过生成的 `mquickjs_ridl_register.h` 中的 `JS_CLASS_*` 常量来确认 class id 语义。

---

## 8. 与既有 class-id 文档/管线的一致性说明

- 本方案复用 ROM/build 的“JS_CLASS_USER + i”分配语义；module class 也应作为 user class 参与该分配。
- V1 仅依赖 `mquickjs_ridl_register.h` 中的 `JS_CLASS_*` 常量；不引入额外的 class-id 导出链路。

---

（完）
