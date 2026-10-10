# B0 Spike 报告：LVGL 能力验证（mquickjs-ui via RIDL）

> 日期：2026-10-10。状态：**完成**。
> 上游规划：[2026-10-10-openvela-ecosystem-plan.md](2026-10-10-openvela-ecosystem-plan.md) Phase B 门槛项。
> 结论先行：**GO —— 建议 B1（UI 封装层正式立项）**。四个风险点全部实机通过，
> 未发现阻断性问题；暴露的都是可排期的功能性缺口（见下）。

## 1. 验证目标与形态

验证「JS（mquickjs，RIDL 绑定）→ LVGL」的完整技术路径，四个风险点实测：

| # | 风险点 | 结果 |
|---|--------|------|
| a | 回调内跑 JS 触发 GC（压缩/JSGCRef 重定位）与 LVGL 无互相伤害 | ✅ 19 次点击 × 每次 2000 字符串分配（~200KB churn/次），回调桥槽位随引擎重定位正常工作，零崩溃 |
| b | LVGL 事件与 JS 引擎同线程 | ✅ 架构即单 task（LVGL 循环驱动一切）；真实 X11 触摸点击与 `lv_obj_send_event` 注入走同一分发路径，行为一致 |
| c | 脚本 eval 返回后回调仍可触发 | ✅ 全部 19 次回调都发生在 `mqjs_rs_ridl_eval` 返回之后（事件循环驱动） |
| d | `LV_EVENT_DELETE` 句柄失效 | ✅ t+12s `lv_obj_del` → DELETE trampoline 调 `mqjs_ui_callback_release` 注销回调桥 JSGCRef 槽位；删除后发送落空、UI 稳活、干净退出 |

**验收产物**：`assets/ui_live.png`（点击 12 次后：标题/信息行/按钮布局正常）、
`assets/ui_deleted.png`（删除后：按钮消失、UI 存活）。sim:mqjs_lvgl（CMake 轨，
X11 fb 640×480）全流程哨兵 `MQJS_LVGL: engine ready → eval ok → … → DONE`。

## 2. 形态（已落地的三仓拓扑）

```
JS: var btn = new Button(); btn.onClick(function(){ title.setText("...") })
      │ RIDL 全局类（编译期注册）
      ▼
mquickjs-ui（external/mquickjs-ui，纯绑定层，零 LVGL 链接依赖）
  Label/Button RIDL 类 + LabelImpl/ButtonImpl
  LvglBackend vtable（repr(C) 函数指针表，attach 注入）
  mqjs_ui_dispatch_click(ctx,handle) / mqjs_ui_callback_release(ctx,handle)
      │ vtable 调用（C ABI）
      ▼
mqjs/lvgldemo（apps/system/mqjs/lvgldemo，适配仓，宿主 app）
  lvgl_backend.c：vtable 四函数 → lv_*；CLICKED trampoline / DELETE trampoline
  mqjs_lvgl_demo.c：LVGL init → 引擎装配(mqjs_rs_ridl_context_new) →
                    attach → eval 内嵌 JS → 统一循环(lv_timer_handler+usleep)
      │
      ├── 引擎 C/stdlib/聚合寄存器：mqjs app 的 CSRCS（源码级集成）
      └── libmqjs.a：Rust 适配层（ctx 三件套 + mquickjs-ui + stdlib 胶水）
```

关键机制：回调回程**显式携带 `JSContext*`**（`ContextToken::from_js_ctx`）。
eval 返回后 `ContextToken::current()` 的 TLS 栈为空，事件回程只能走
from_js_ctx——这是本 spike 对回调桥设计的关键修正。

## 3. 过程中修复的缺陷（工程教训）

1. **ridl-tool：连字符 crate 名生成非法 Rust 路径**——聚合 bootstrap 模板
   原样发射 `mquickjs-ui::initialize_module()`（E0425）。修复：发射前经
   `sanitize_ident` 规范化为 lib target 名。回归测试
   `aggregate_bootstrap_crate_ident_test`（demo 仓 + SDK vendored 拷贝同步）。
2. **mquickjs-ui 独立构建三连**：`&str::into_bytes`（不存在）、edition 2024
   的 `#[unsafe(no_mangle)]` 缺 unsafe、类 id 链（独立测试需要含本 crate
   类 id 的 app 框架产物 + stdlib 链接）→ `scripts/run-tests.sh` +
   `build.rs` 按 `MQJS_ENGINE_LINK` 门控的 stdlib 链接 +
   `ridl_stdlib_lib_path()` helper（ridl-glue，两仓同步）。
3. **`lv_event_send` vs `lv_obj_send_event`**：LVGL 9.1 的 `lv_event_send`
   是 `(list, e, preprocess)` 内部形态——按 8.x 习惯传 `(obj, code, param)`
   会把 code(=7) 当指针（SIGSEGV，现场 `e=0x7`）。对象级入口是
   `lv_obj_send_event`。
4. **回调内 console.log 静默失效**：stdlib `DefaultConsoleSingleton` 用
   `ContextToken::current()`（TLS）取 ctx——回调上下文栈空 → 静默 no-op。
   修复：mquickjs-ui 回调桥 invoke 期间 `with_current` 压栈（语义正确：
   回调执行时该 context 即当前 context）。SDK 侧根治（console impl 改用
   env 携带的 ctx）列入缺口清单跟进。
5. **构建系统三坑**：
   - sim:mqjs_lvgl 需 `CONFIG_FS_LINKS=y`——它提供 `readlink`（经
     nuttx-names.dat 重定义为 NXreadlink），Rust std 的 readlink 才能解析；
   - CMake/Make 的 cargo 依赖监听必须覆盖**路径依赖源**（SDK crates +
     mquickjs-ui），否则改 mquickjs-ui 后 cargo 不重跑、镜像链旧归档（本次
     console 修复第一次未生效的根因）；
   - `build.sh -b` 会整体覆盖 cmake 二进制目录，不带 board_config 后缀。

## 4. RIDL 缺口清单（B1 的输入，按阻塞度排序）

| # | 缺口 | 现状/影响 | 建议 B1 优先级 |
|---|------|-----------|---------------|
| 1 | **布局/样式 API**（align/pos/size/style） | demo 排版靠宿主 C 侧兜底（`demo_layout`）；JS 无法布局 | P0：`align(x,y[,parent])`/`setSize` 最小面 |
| 2 | **脚本载体**：demo JS 内嵌 C 字符串 | 正式应用需要文件/ROM 脚本装载 + 多文件模块 | P0（复用 `require()` 即可低成本满足） |
| 3 | **组件树/容器**：无容器类，全部挂在默认屏 | 页面结构无从建起；`LV_EVENT_DELETE` 级联失效需容器语义 | P1：Container/Screen 类 + 父子关系 |
| 4 | 回调形参：v1 白名单 bool/i32/f64/string | LVGL 事件常需坐标/键值等结构化参数 | P1：`callback(x: i32, y: i32)` 多参（桥已支持多参，codegen 面放开） |
| 5 | 对象引用传递：方法不能收/返组件对象（`getLabel()` 之类） | 组件组合受限；`Traced<JSValue>` 机制已备（gc_traced 先例） | P1：`class` 返回值/参数形态 |
| 6 | 生命周期：JS 对象 GC 后 LVGL 对象不回收（反向由 DELETE 桥接） | 长跑应用内存增长；需 finalizer → lv_obj_del 桥 + 归属容器约定 | P1 |
| 7 | 更多 widget（Image/Input/滚动容器…） | 机械扩展，机制已验证 | P2 按 app 需求 |
| 8 | SDK 侧 console impl 的 TLS current 依赖 | 桥已补 `with_current` 语义；根治应改 env-ctx | P2（SDK 小改） |
| 9 | 声明式组件树（快应用 lite 形态） | mquickjs-ui README 演进路线 3 | B1 之后 |

## 5. 对 B1 的 go/no-go 建议

**GO**。依据：

- 机制面：编译期注册（RIDL 全局类）→ vtable 后端 → C trampoline →
  ctx 显式回程 → 同步回调桥，全链路在真实 LVGL/sim 环境稳定工作；
- GC 面：tracing GC + JSGCRef 槽位在持续分配压力（19×2000 字符串）下无
  重定位事故——mquickjs（非 QuickJS）的 GC 模型与回调桥设计互相印证；
- 工程面：三仓拓扑（SDK / mquickjs-ui / 适配层）构建、测试、再生成链路
  全部打通且可复现（SYNC.md）；
- 缺口全部为**功能性、可排期**，无阻断性架构风险。

建议 B1 范围（P0）：布局/样式最小面 + 脚本文件装载 + Container 类，
目标是在 sim 上做出一个可翻页的多页面 demo（对齐「类快应用」形态）。

## 6. 变更清单（本 spike 落盘）

- **mquickjs-ui**（main）：ctx 版 dispatch/release、`mqjs_ui_backend_attach`、
  with_current 语义、构建/测试链修复（`scripts/run-tests.sh`）、README。
- **mquickjs-openvela 适配仓**（mqjs-in-tree）：`lvgldemo/`（嵌套 app：
  Kconfig source / Make.defs 注册 / CMake add_subdirectory）、gen/ 再同步
  （含 Label/Button 聚合 + atom/stdlib 头 + 框架产物）、双轨 cargo 依赖
  监听修复、SYNC.md 增补。
- **SDK**（master）：vendored ridl-tool 连字符 crate 名修复 + 回归测试、
  ridl-glue `ridl_stdlib_lib_path` helper。
- **demo 仓**（本仓）：ridl-tool 同款修复 + `aggregate_bootstrap_crate_ident_test`。
- **nuttx fork**：`boards/sim/sim/sim/configs/mqjs_lvgl/defconfig`。
