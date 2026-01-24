# V1 规范范围（本仓库解释版）

本文档用于明确：本仓库当前一轮迭代的 **测试目标 = V1 合规**。

- V1 缺口与执行顺序：见 `docs/planning/2026/2026-01-24-v1-gap-todo-list.md`

- **V1 合规**：对 V1 规范内声明“应支持”的语义与类型组合，提供端到端可回归的用例（`tests/global/**`）。
- **V2**：整体能力尚未实现、或超出 V1 的能力（例如 module 模式），明确不在本轮范围。

> 说明：本文档不是外部标准的复刻，而是为了让 `ridl-builder/codegen/runtime glue` 的改动可以对齐到可执行的回归目标。


## 1. 两类缺口的处理规则

### 1.1 不在 V1 的整体能力（归入 V2）
满足以下任一条件，视为 **V2**：

- 需要新的“注册/导出/可见性”模型，而不是单纯的类型转换或 glue 行为补齐。
- 需要新的解析/聚合策略（例如多模块分包、按模块命名空间隔离）。

典型示例：
- **module 模式**（按 module 注册/导出、import/using 的模块化语义）

处理方式：
- 不在本轮新增/执行对应测试用例。
- 文档中明确写为 V2。

### 1.2 属于 V1 但当前实现未支持/行为不一致（必须补用例）
满足以下条件，视为 **V1 缺口**：

- 该类型/语义属于 V1 范围。
- 当前实现不支持、或行为与预期不一致（含 strict/default 的差异）。

处理方式：
- **必须新增测试用例**（通常落在 `tests/global/types`，或按语义域拆到对应 test module）。
- 测试先红后绿，作为本轮回归基线。


## 2. 本轮测试目标（V1 合规）

本轮的“完成标准”是：
- `cargo run -p ridl-builder -- prepare`
- `cargo run -- tests`
- `cargo test`

均通过，且覆盖清单中的用例均存在并稳定运行。

### 2.1 V1 module(require) 的最小完成标准

> 说明：V1 的 module 仅覆盖 **require 形态**；import/using 的 module 语义不在 V1。

- require 能加载 module：`require("<module_path>@<version>")`
- module exports 至少支持：function / class
- class exports 在 JS 侧可 `new`，并具备 prototype 方法
- 跨导出互操作：
  - 多 class 场景下，A 返回 B、A 接受 B 等行为正确
  - 导出顺序不影响可用性（不依赖注册顺序的偶然性）

#### 测试入口

- `tests/module/smoke/**`：最小 smoke
- `tests/module/basic/**`：module_basic 的互操作覆盖
- `tests/module/exports/**`：导出形态覆盖（例如 single-class export）
- `tests/module/rom/**`：require/materialize 的可观测行为（弱断言）


## 3. 目录与测试组织约定

- 框架级测试模块（framework-level）统一在：`tests/global/<domain>/test_<domain>/`
  - `src/*.ridl`：该语义域最小化 RIDL 输入
  - `src/*_impl.rs`：手写实现生成 trait
  - `tests/*.js`：JS 端到端用例（与 RIDL module 同目录）

- 功能模块（例如 stdlib）仍属于：`ridl-modules/stdlib`（以及未来的其他功能模块目录）


## 4. V1 vs V2：当前仓库的明确划分

### 4.1 明确仍归入 V2（本轮不支持）

- import/using 的 **module 语义**（按 module 命名空间隔离、ESM import 互操作等）

> 说明：本仓库当前已实现 module(require) 形态，因此 **module 不再归入 V2**。
> 对应现状：`tests/global/import_using/test_import_using/src/test_import_using.ridl` 中的注释已更新为：
> “after V1 defines import/using module semantics”。

### 4.2 V1 范围内（本轮必须逐步补齐）

- global 模式下的类型系统与 glue 行为（`tests/global/types` 为主）
- strict/default 等模式下的调用校验差异（属于 V1 的部分）
- literals/js_fields 等在 V1 内的语义（若受聚合链路限制，可用“方法返回值/参数”过渡，但最终仍需回归到规范支持的成员形式）
- **module 模式（require 形态）**：module 注册/导出/可见性与 require 行为（`tests/module/**` 为主）
  - 命名规则以 RIDL 语法为准：`module_path = identifier ("." identifier)*`，不支持 `-`。
  - require() 不做 `-`/`.` 的归一化映射；传入不符合语法的 module id 应失败。


## 5. tests/global/types：V1 覆盖目标清单（逐步补齐）

> 目标：让 `test_types` 不只是验证 `any` 透传，而是成为 V1 类型系统的回归基线。

### 5.1 当前已覆盖（现状）

- `any` 作为参数：`TestTypes.echoAny(v: any)`
- JS 用例：`tests/global/types/types_nullable.js` 目前仅覆盖 `echoAny(null)`

README 里还声明了“nullable/union”等目标，但 RIDL 目前尚未落地相应用例。

### 5.2 本轮应补齐的 V1 用例方向（建议矩阵）

下面是建议按“类型 × 位置（参数/返回/字段）× 模式（strict/default）”组织的用例方向。
具体条目落地时以 V1 的精确定义为准（你确认后我再把它细化成可执行清单）。

- 基础类型：`bool/int/float/string`
  - 参数传入合法值/非法值（strict 下应报错）
  - 返回值类型正确性

- nullable：`T?`
  - 允许 null 的参数/返回
  - strict/default 差异（若 V1 定义存在差异）

- any
  - default 模式下作为参数/返回
  - strict 模式下（是否允许、仅 variadic 允许等）

- union：`A | B` 以及包含 null 的 union（如 `A | B | null`）
  - 规范化行为（如果 V1 有“语法等价/规范化”定义）

- （待确认属于 V1 的）复合类型：array/map/tuple/struct/enum/msgpack struct
  - 以最小化输入覆盖 parser/generator/glue 的端到端路径


## 6. 现有已知限制（需要明确归类）

### 6.1 mode 语法限制（当前实现现状）

`tests/global/types/test_types/README.md` 提到：
- ridl-tool 仅支持 `mode strict;`
- 但 strict 禁止非 variadic 参数使用 `any`
- 因此 `test_types` 目前省略 mode_decl，走默认 FileMode::Default

这条需要在“V1 合规”口径下明确：
- 如果 V1 允许/要求显式 `mode default;`：则这是 **V1 缺口**，需要补齐语法与用例。
- 如果 V1 不要求显式 default（或允许省略）：则可作为过渡，但仍建议用例中覆盖 strict/default 的边界。


## 7. V1 合规实施顺序（依赖顺序：基础 → 组合）

按功能依赖从基础到高级推进，避免在高层用例里“顺带发现类型系统缺口”导致定位困难：

1. **Phase A：`tests/global/types/test_types`（default）**
   - 先把 V1 类型系统/转换语义做成可回归基线。

2. **Phase B：`tests/global/diagnostics/test_diagnostics`（strict）**
   - 固化 V1 strict 校验与报错路径，让“应失败”也可回归。

3. **Phase C：形态 smoke（只使用 Phase A 已覆盖的类型）**
   - `tests/global/singleton/test_singleton`
   - `tests/global/fn/test_fn`
   - `tests/global/class/test_class`

4. **Phase D：成员/字面量/扩展能力（可能依赖聚合链路与语法支持）**
   - `tests/global/literals/test_literals`
   - `tests/global/js_fields/test_js_fields`

5. **Phase E：V2 占位（本轮不扩展）**
   - `tests/global/import_using/test_import_using`


## 8. 下一步（为了把本文档变成可执行目标）

从 Phase A 开始，把 `test_types` 的“类型矩阵”细化成可执行清单：
- 每个用例：对应的 RIDL 声明 + JS 断言 + 预期（PASS/FAIL）
- 你审阅确认后再开始实现与修复。
