# V1 功能缺口 TODO LIST（由易到难）

> 日期：2026-01-24
>
> 背景口径：V1 = global + module(require) 形态。
> - module 命名规则以 RIDL 语法为准：`module_path = identifier ("." identifier)*`，不支持 `-`。
> - V1 **不**纳入 import/using 的 module 语义。
>
> 目标：把当前仓库在 V1 范畴内“还没做/没做完/语义不完整”的点列成可逐项勾选的 TODO。
> 每项都给出完成判据（至少含测试命令）。

---

## 0. 总体验收命令（每个阶段完成后都应通过）

- `cargo run -p ridl-builder -- prepare`
- `cargo test`
- `cargo run -- tests`

---

## 1. 完全未实现（解析/生成/用例都没有）

> 说明：这些要么不在 V1（明确延后），要么缺“可观测断言面”，因此现阶段无法写成可回归用例。

- [ ] **[V1-A0] import/using 的 module 语义（明确不在 V1）**
  - 状态：V1 范畴外（作为 V2/未来工作项占位）。
  - 完成判据：不适用（除非将来把该语义纳入 V1/V2，再改成可执行 TODO）。

- [ ] **[V1-A1] ROM2/N1：class id / ROM index 的稳定可观测断言面**
  - 痛点：目前 JS 侧不可直接读取 class id/ROM index；仅靠行为测试无法对齐 ROM join。
  - 候选方案（需先选定其一）：
    1) 通过生成物断言：解析 `target/ridl/.../ridl-manifest.json` / `mquickjs_ridl_register.h` / 其他稳定输出；
    2) 暴露 debug API：例如 `__ridl_debug.*` 只读表（会影响运行时面）。
  - 完成判据：
    - 新增用例（JS 或 Rust）能稳定断言“module 内多 class 的 ROM class index join 与生成的 class id 对齐”；
    - `cargo run -- tests` 覆盖到该断言。

---

## 2. 部分实现（存在解析/部分生成，但闭环缺失或被禁用）

### 2.1 语法在 grammar 里存在，但 parser/语义层显式禁用

- [x] **[V1-B0] singleton 的 normal_prop（已删除残留语法）**
  - 结论：这是残留语法/实现不一致点，应彻底去除。
  - 已处理：从 RIDL grammar 中移除 singleton_member 的 `normal_prop`，并删除 singleton parser 对该分支的特殊处理。
  - 验收：全量命令通过（见 §0）。

> 备注：原先的 `const`（class const/const_member）不纳入本仓库 V1 语法目标。
> 原因：JS 引擎无“const 变量/绑定”语义；且本仓库不做语法兼容实现，因此保持 parser 拒绝即可。

### 2.2 类型系统：解析/normalize 存在，但 Rust typed boundary 不完整

- [ ] **[V1-B2] union 的 Rust 类型生成（typed param/return）**
  - 现状：Type::Union 可解析；normalize 存在；但 `rust_type_from_idl(Type::Union)` 未实现。
  - 完成判据：
    - 允许在函数参数/返回值位置出现 union（含 Optional(union)）；
    - 新增 tests（global/types + module/basic）覆盖：
      - union param decode（至少 2-3 成员类型）
      - union return encode
      - Optional(union)（`null`/`undefined` 路径）
    - 全量命令通过（见 §0）。

- [ ] **[V1-B3] Custom 命名类型（using alias）作为 typed param/return**
  - 现状：Type::Custom 可解析，但 V1 glue 明确不支持。
  - 预期：至少支持 alias 到基础类型/容器类型（例如 `using UserId = int`）。
  - 完成判据：
    - alias 可在 param/return 位置使用且不触发 compile_error；
    - tests 覆盖 alias param/return 的 encode/decode；
    - 全量命令通过（见 §0）。

- [ ] **[V1-B4] singleton 的 var/proto var（若要纳入 V1）**
  - 现状：grammar 不允许（singleton_member 未包含 var_member/proto_var_member）。
  - 备注：此项是否属于 V1 需要单独确认；若纳入则工作量偏大。
  - 完成判据：
    - 支持 singleton 内 var/proto var 的 parse + generate + runtime 安装；
    - tests 覆盖实例/原型可观测行为；
    - 全量命令通过（见 §0）。

---

## 3. 已实现但语义不完整（可跑但覆盖面有限/存在硬兜底）

- [ ] **[V1-C0] 消灭 v1 glue 的 compile_error 兜底（按格子补齐类型覆盖）**
  - 现状：生成器存在 `compile_error!("v1 glue: unsupported …")` 分支，说明类型/位置覆盖不完整。
  - 目标：把“最常用类型 × 位置”的格子逐个补齐，逐步缩小兜底面。
  - 建议拆分子任务（示例）：
    - [ ] C0-1：nullable string/int（param/return/property）
    - [ ] C0-2：array<primitive>（param/return）
    - [ ] C0-3：map<string, primitive>（param/return）
    - [ ] C0-4：object（param/return）
    - [ ] C0-5：variadic 参数对上述类型的支持
  - 完成判据：
    - 每补一个格子都必须新增/更新 tests；
    - 对应 `compile_error!` 分支在该格子上不再可达；
    - 全量命令通过（见 §0）。

- [ ] **[V1-C1] strict 模式下 any 的位置限制：规则与用例补齐**
  - 现状：validator 有规则，但覆盖矩阵可能不完整。
  - 完成判据：
    - 用例覆盖 strict 下 any 在：非 variadic param、variadic param、return 等位置的允许/禁止；
    - 错误诊断稳定（至少能断言“会抛错/会拒绝构建”）。

---

## 4. 推荐执行顺序（由易到难）

1) **B0（语法禁用点）**：singleton normal_prop
   - 原因：改动范围清晰、反馈快；能让语法/文档/实现先对齐。
2) **C0（类型覆盖格子）**：从最常用类型开始消灭 compile_error 兜底
   - 原因：每步都能落用例，回归价值高。
3) **B2（union typed）**
   - 原因：牵涉 enum 生成、decode/encode、Optional(union) 等组合复杂度更高。
4) **A1（ROM2/N1 可观测面）**
   - 原因：需要先选定“断言面”，可能影响产物稳定性与工程结构。

---

## 5. 当前已纳入 V1 的 module(require) 现状（便于对照）

- tests/module/smoke/**
- tests/module/basic/**
- tests/module/exports/**
- tests/module/rom/**

> 对 module 的命名字符集/归一化：已在 `tests/module/exports/test_module_single_class/tests/naming_ids.js` 固化当前行为。
