# OpenVela 生态集成方案（v2，对抗复核后修订）

> 状态：v1 经对抗复核判定"修改后通过"（3 Critical / 4 Important），本 v2 按复核意见重排。
> 日期：2026-10-10

## v2 修订摘要（对抗复核吸收）

复核发现的三个 Critical 及处置：

- **C1 上游落树形态是隐含的最大决策**：mqjs 在 openvela apps 仓**未跟踪**
  （本地嵌套 clone）、`external/mquickjs-rs-sdk` **不在上游 manifest**、构建强依赖
  cargo+libclang——tests PR 若先行，在上游是不可编译启用的死代码。
  → 新增 **阶段 0：落树形态决策**（推荐：C-only vendored 进
  `apps/interpreters/mquickjs`，完全镜像 apps/interpreters/quickjs 官方先例，
  js_main_c.c 已存在即 C-only 入口；Rust 适配器作为后续增量 PR 带 CI 证据再上）。
- **C2 阶段 B 与本仓知识库决策冲突**：`docs/knowledge/decision_abandon_ui_app_framework.md`
  （2026-09-04）四路复核否决过 JS-UI 方向并留下重新考虑前置条件（具名需求方、
  应用分发机制、一日 spike、事件循环设计先行）。v1 通篇未引用——违反本仓知识纪律。
  → 阶段 B **降级为门槛触发**：须先逐条回应旧决策前置条件（绑定模块 ≠ 框架的
  反驳是否成立由用户裁定），未触发前不启动。
- **C3 RIDL 回调支持是 no-op stub**：filters.rs 把 Callback 映射为空闭包
  （"callback will be invoked by async bridge"），异步桥是 fire-and-forget 形状，
  不符合 LVGL 事件需要的同步 many-shot 回调；且每个 LVGL C 函数仍需手写 Rust
  FFI shim（"自动生成"言过其实）；LVGL 对象 **lifetime 失效**（C 侧删除后 JS
  句柄悬空）是缺口清单漏掉的第四项。
  → 若 B 启动：手写最小绑定先行，RIDL 扩展以工作实现为参照事后回填。

Important 吸收：tests 驱动模型改"进程内直调 js_main（无 exit）+ pytest 控制台
断言"（system() 拿不到子进程 stdout 且回传值不可靠）；lvgldemo_test 不是
cmocka（全 testcases 仅 media_test 用 cmocka），mqjs-citest 需补
SYSTEM_SYSTEM/SCHED_HAVE_PARENT 等；ai_agent 的 `tool_registry_register_provider`
**已是运行时动态注册**（两个消费者 + MCP 远程工具先例）——C0 机制问题已有答案，
改问价值差异化；贡献对象修正为独立上游仓 `open-vela/packages_ai_agent`；
树内 quickjs 实为 2021-03-27；lvgl_fb 无 libuv（手动 while 循环）→ 事件桥
定案方向"LVGL 回调只入队、主循环统一 drain"（与 completion-queue 同构，B0
难度下调）。
> 背景：mquickjs SDK（pengyz/mquickjs-rs-sdk）+ 适配层（pengyz/mquickjs-openvela）
> 已完成 openvela 接入（sim/QEMU aarch64 双目标、Make/CMake 双构建、四哨兵全绿）。
> 生态调研结论：openvela 的既定 JS 格局 = quickapp 闭源 UI 运行时（16MB heap 门槛）
> + 2020 版树内 QuickJS（绑定空白）；**开源、轻量、带自动绑定工具链的 JS 通道空白**；
> ai_agent 是官方战略第一方向；官方测试框架无任何 JS 用例。

## 0. 总原则

- **上游优先**：所有产出先落我们 fork，达标后向 open-vela org `dev` 分支提 PR
  （签 CLA、原子提交、Apache 2.0 头、带测试——CONTRIBUTING 硬要求）
- **不做闭源竞品替代**：不碰 quickapp；定位"开源轻量 JS 通道"，与 quickapp 高低互补
- **每阶段独立可交付**：任一阶段中止，前面的产出仍独立有价值
- 仓库流：SDK 与适配层继续走 pengyz fork；贡献上游的补丁从 fork 分支提起

---

## 阶段 A：mqjs 进官方测试框架（②，规模：≤1 周）

### 目标

把 mquickjs 变成 openvela 官方测试/CI 体系的一部分——全树目前**没有任何
JS testcase**，这是回馈上游接受度最高的形态，同时为后续阶段建立官方 CI 背书。

### 非目标

不改 SDK/适配层功能；不做性能基准（后续机会⑥单列）。

### 交付物

1. **`tests/testcases/mqjs_test/`**：cmocka 形态 testcase app
   （Kconfig + CMakeLists.txt + .mk + mqjs_test.c），复刻 lvgldemo_test 的
   结构标准。用例设计：
   - `mqjs_test` 内部以 `system("js ...")` 或直接链接 libapps 方式驱动
     `js` builtin 跑 `ports/openvela/cases/` 四哨兵
   - 断言哨兵输出（`CASE x PASS` / `CASES n/m PASS`）；tiny_err 断言
     "SyntaxError 上报且进程不崩"
   - cases 脚本需随用例进树（tests/testcases/mqjs_test/cases/ 或复用
     apps/system/mqjs 内置脚本——执行时确认脚本分发方式）
2. **`tests/scripts/script/test_mqjs/test_mqjs.py`**：pytest 驱动用例
   （对齐 test_example 模式），覆盖 sim 与 QEMU 两种执行环境
3. **citest defconfig 变体**：`boards/sim/sim/sim/configs/mqjs-citest/`
   （mqjs + TESTING_CMOCKA，对齐 citest 家族形态），上游评审时的演示配置
4. **上游 PR**：apps（若有改动）+ tests 两仓，先 Issue 后 PR（贡献规范）

### 验收

- [ ] sim:mqjs-citest 配置构建通过，`mqjs_test` NSH 命令运行四哨兵全 PASS
- [ ] pytest 用例在本机（sim）跑通；QEMU 路径以文档说明 + 预留 hook 交付
- [ ] PR 材料齐备（CLA 已签、commit 原子、许可证头）

### 依赖与风险

- 依赖：tests/ 框架现成（cmocka + pytest），无新基础设施
- 风险：cmocka 用例与 `js` builtin 的进程内/进程外驱动方式需按树内先例
  （lvgldemo_test 是直接链接库形态）确认——**第一项任务即研读 2-3 个既有
  testcase 的驱动模型**再定实现

---

## 阶段 B：JS→LVGL 绑定层 mqjs-lvgl（①，规模：2-4 周）

### 目标

用 RIDL 描述 LVGL 9.1 核心子集，自动生成 mquickjs↔LVGL 绑定，让 `js`
命令在带屏目标（sim lvgl_fb / goldfish）上创建 UI、响应事件——填补
"开源轻量 JS-UI 通道空白"，并成为 RIDL 工具链的标杆应用场景。

### 非目标

- 不做完整 LVGL API 覆盖（9.1 API 面极大，只做核心子集）
- 不做快应用兼容（RPK/路由/生命周期是 quickapp 领域）
- 不引入新 UI 框架

### 里程碑

**B0. Spike：JS 引擎与 LVGL 事件循环共存**（3-5 天）
- 目标：sim lvgl_fb 配置下，`js` builtin 与 LVGL demo 同镜像运行；
  弄清三件事：① LVGL 的 lv_timer/lv_nuttx_uv_loop 与 mquickjs 的
  async drain（completion queue）如何共存于主循环；② LVGL 句柄
  （lv_obj_t*）经 RIDL 传值的类型映射方案（指针类不透明句柄 = 现有
  opaque 机制的逆方向：C 对象→JS 包装）；③ RIDL 现有类型系统缺口清单
  （回调函数指针、对象方法、继承/父类样式）
- 交付：可行性结论 + RIDL 扩展需求清单 + 演示 commit（JS 建 label 显示文字）
- **门槛**：此 spike 不过则 B 重估（备选：绕开 RIDL 手写最小绑定先证明价值）

**B1. RIDL-LVGL 核心子集**（1-1.5 周）
- 覆盖：screen/obj 创建与树操作、基础 widget（label/btn/arc/bar）、
  style 基础（bg/文字/对齐）、事件注册（JS 闭包→LVGL 事件回调桥）、
  lv_timer 桥接
- 事件桥是技术核心：JS 函数被 C 回调 → 复用 SDK 异步桥（completion
  queue + drain）或直接同步调用（需评估重入安全）， spike 定案
- 交付：mqjs-lvgl RIDL 模块（进 SDK ridl-modules/）+ 绑定生成通过 + 
  sim lvgl_fb 上 JS 建 UI + 按钮点击回调改 label 文本的端到端 demo

**B2. Demo + 文档 + 上游 Issue**（3-5 天）
- 用 B1 重写一个官方小 demo（wooden_fish 或 calculator）作为对照布道
  （JS 版 vs C 版行数对比），产出 `docs/zh-cn/app_dev` 教程草稿
- 向 open-vela 提 Issue（新功能先讨论规范）+ PR（绑定模块本体）

### 验收

- [ ] B0 结论文档 + JS 创建 label 的演示
- [ ] B1 端到端：JS 创建按钮 + 点击回调更新文本（sim lvgl_fb）
- [ ] B2 demo 对照 + 教程草稿 + Issue/PR 材料齐备

### 依赖与风险

- 依赖：apps/graphics/lvgl 9.1（树内）、sim lvgl_fb defconfig、SDK 异步桥
- 风险 1（高）：LVGL 事件回调与 mquickjs GC 的交互——回调持 JS 闭包需
  Root/Traced 保护（SDK 的 JSGCRef 体系正好派上用场，是真实场景检验）
- 风险 2（中）：RIDL 类型系统缺口（句柄/回调/方法语义）可能需要扩展
  RIDL——B0 spike 先出清单，避免边做边改语法
- 风险 3（低）：LVGL 线程模型（默认非线程安全）→ 单线程主循环约定规避

---

## 阶段 C：ai_agent 的 JS 工具/技能运行时（③，规模：3-5 周，含研究）

### 目标

把 mquickjs 定位为 openvela 端侧 AI Agent（官方战略第一方向，~256KB RAM
档位）的**工具/技能脚本运行时**：agent 的工具与技能用 JS 编写，经 RIDL
调用设备 API——JS 生态的开发效率 × mquickjs 的内存档位 × RIDL 的安全绑定。

### 非目标

- 不做 LLM 推理侧任何事（推理在云端/独立 NPU 路径）
- 不替换 ai_agent 的核心 ReAct 框架（C 实现）
- 阶段 C0/C1 是研究+PoC，是否深入由 PoC 结论决定

### 里程碑

**C0. 接口研究**（3-5 天）
- 研读 `packages/ai_agent/`：agent_skills 的技能编写规范、include/ 的
  工具注册 API、工具调用的数据流（谁调谁、参数怎么传、结果怎么回）
- 产出：接口调研报告 + mquickjs 契合点/缺口分析 + PoC 方案
- **门槛**：若 ai_agent 工具注册是纯编译期 C 宏且无动态加载缝隙 →
  JS 运行时价值降级，向用户汇报后再定去留

**C1. PoC：一个 JS 工具接入 agent**（1-1.5 周）
- 按 C0 方案实现：一个最小工具（如设备信息查询）以 JS 编写、经
  mquickjs 执行、结果回 agent 工具调用链
- 交付：PoC commit + 演示 + 决策材料

**C2. 产品化**（视 C1 结论，1-2 周）
- 技能加载规范（JS 技能包格式）、内存档位验证（256KB 目标下的
  heap 配置实测）、文档

### 验收

- [ ] C0 调研报告 + 门槛判定
- [ ] C1 端到端 PoC（agent 调用 JS 工具拿到正确结果）
- [ ] C2（若继续）：技能包格式文档 + 内存实测数据

### 依赖与风险

- 依赖：阶段 A 的测试设施（agent 场景同样需要回归保障）、阶段 B 可选
  （agent 工具若涉及 UI，JS-LVGL 直接复用）
- 风险 1（中）：ai_agent 接口封闭（编译期注册、无动态缝隙）→ C0 门槛判定
- 风险 2（中）：256KB heap 档位对 mquickjs 最小配置的考验（无 JS heap
  实测数据）→ C1 顺带产出数据

---

## 时间线总览

```
第 1 周      阶段 A（测试回馈，含上游 PR 流程启动）
第 2-3 周    阶段 B0+B1（spike + 核心子集）     ← 与 A 尾部并行
第 4 周      阶段 B2（demo+文档+Issue）＋ C0 启动
第 5-7 周    阶段 C（研究→PoC→决策）
```

关键决策点：
- D1（第 2 周末）：B0 spike 门槛——RIDL 缺口清单是否可控
- D2（第 4 周末）：C0 门槛——ai_agent 接口是否有动态缝隙
- 每个决策点向用户汇报后继续

## 风险总表

| 风险 | 等级 | 对策 |
|------|------|------|
| LVGL 事件回调 × GC 交互复杂 | 高 | 复用 SDK JSGCRef 体系；B0 spike 先行验证 |
| RIDL 类型系统缺口（句柄/回调） | 中 | B0 出清单，扩展 RIDL 作为独立前置任务评审 |
| ai_agent 接口封闭 | 中 | C0 门槛判定，不合适即止损 |
| 上游 PR 接受度（B 的绑定模块体量大） | 中 | A 先行建立信任；B 拆小 PR（先绑定框架后 widget 增量） |
| mquickjs 与树内 quickjs 的定位冲突质疑 | 低 | 官方文档口径：高低搭配（MCU 级 vs 产品级），基准数据（机会⑥）补充论证 |
| 测试框架驱动模型不明 | 低 | 阶段 A 首任务研读先例 |

## 与现有资产的衔接

- 四哨兵 → 阶段 A 用例素材（现成）
- RIDL 自动绑定 → 阶段 B 核心武器（现成）
- SDK 异步桥/JSGCRef → 阶段 B 事件桥 + 生命周期保护（现成）
- no_std/aarch64 能力 → 阶段 C 内存档位潜力（现成）
- 本仓的 AI 纪律体系（知识沉淀/对抗复核）→ 全程沿用
