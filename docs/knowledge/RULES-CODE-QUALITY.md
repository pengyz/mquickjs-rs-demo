---
id: code-quality-rules
version: "1.0"
updated_at: "2026-09-03"
---

# 代码质量检查规则（Rust 生态适配）

## 支持的语言

| 语言 | 扩展名 | rg --type |
|------|--------|-----------|
| Rust | rs | rust |
| JavaScript | js | js |
| C/C++ | c, h | c, cpp |

## 安全类 (security)

```yaml
- id: S01
  name: unsafe 块无安全注释
  severity: high
  check_command: |
    rg "unsafe\s*\{" -A 2 -g '*.rs' | rg -v "//.*Safety|/\*.*Safety"
  fix_hint: 每个 unsafe 块必须有 Safety 注释说明为什么安全

- id: S02
  name: 裸指针解引用
  severity: critical
  check_command: |
    rg "\*\s*(const|mut)\s+[a-zA-Z_]" -g '*.rs'
  fix_hint: 裸指针操作必须在 unsafe 块内，并说明生命周期保证

- id: S03
  name: transmute 使用
  severity: high
  check_command: |
    rg "std::mem::transmute|transmute::<" -g '*.rs'
  fix_hint: transmute 极易引入 UB，优先用 safe 替代（as、from_raw_parts 等）

- id: S04
  name: FFI 函数缺 Safety 文档
  severity: high
  check_command: |
    rg "extern \"C\" fn" -A 1 -g '*.rs' | rg -v "/// Safety"
  fix_hint: extern "C" 函数必须在文档注释中说明 Safety 约束

- id: S05
  name: 手动管理 JSValue（QuickJS 特定）
  severity: critical
  check_command: |
    rg "JS_FreeValue|JS_DupValue" -g '*.rs'
  fix_hint: 引擎使用 tracing GC，不应手动 free/dup JSValue
```

## 健壮性 (robustness)

```yaml
- id: R01
  name: unwrap/expect 使用
  severity: medium
  check_command: |
    rg "\.unwrap\(\)|\.expect\(" -g '*.rs'
  fix_hint: 使用 ? 或 match 处理 Result/Option，仅在逻辑保证的地方用 expect

- id: R02
  name: panic!/unreachable! 使用
  severity: medium
  check_command: |
    rg "panic!|unreachable!\(\)" -g '*.rs'
  fix_hint: 库代码应返回 Result，仅在真正不可达或测试代码中使用

- id: R03
  name: 索引越界风险
  severity: medium
  check_command: |
    rg "\[[0-9]+\]|\[.+\.\.]" -g '*.rs' | rg -v "// SAFETY|// safe"
  fix_hint: 使用 .get() 或边界检查，避免直接索引

- id: R04
  name: 生命周期省略可能导致悬垂引用
  severity: high
  check_command: |
    rg "fn.*<'_>|&'_ " -g '*.rs'
  fix_hint: 明确生命周期标注，尤其在 FFI 边界

- id: R05
  name: 跨 Context 使用 Root（项目特定）
  severity: critical
  check_command: |
    rg "Root::new" -A 5 -g '*.rs' | rg -v "ctx_id.*scope"
  fix_hint: Root::new 必须校验 ctx_id 相同，防止跨 Context 使用
```

## 性能 (performance)

```yaml
- id: P01
  name: 循环中重复分配
  severity: low
  check_command: |
    rg "for .* in .*\{\s*let.*Vec::new|String::new" -g '*.rs'
  fix_hint: 在循环外预分配或使用 with_capacity

- id: P02
  name: 克隆大对象
  severity: low
  check_command: |
    rg "\.clone\(\)" -g '*.rs' -A 1 | rg "Vec|String|HashMap"
  fix_hint: 考虑借用、Arc 或按需移动所有权

- id: P03
  name: 无限循环风险
  severity: high
  check_command: |
    rg "loop\s*\{|while\s+true\s*\{" -g '*.rs'
  fix_hint: 确保有明确的退出条件或超时机制
```

## 可维护性 (maintainability)

```yaml
- id: M01
  name: 硬编码配置
  severity: low
  check_command: |
    rg "localhost|127\.0\.0\.1|:3000|:8080" -g '*.rs'
  fix_hint: 使用环境变量或配置文件

- id: M02
  name: 魔法数字
  severity: low
  check_command: |
    rg "[=<>]\s*\d{3,}[^0-9]" -g '*.rs'
  fix_hint: 提取为命名常量

- id: M03
  name: TODO/FIXME 标记
  severity: info
  check_command: |
    rg "TODO|FIXME" -g '*.rs'
  fix_hint: 确保有对应的 issue 或计划清理
```

## 项目特定规则

### RIDL 模块约定

```yaml
- id: RIDL01
  name: RIDL 模块缺少 edition = "2024"
  severity: high
  check_command: |
    find tests/global tests/module -name Cargo.toml -exec sh -c '
      grep -L "edition.*2024" "$1" && echo "$1: missing edition 2024"
    ' _ {} \;
  fix_hint: RIDL 模块必须设置 edition = "2024"

- id: RIDL02
  name: 硬编码模块名
  severity: critical
  check_command: |
    rg "\"GLOBAL\"|\"console\"" -g 'ridl-*.rs' -g '**/ridl_*.rs'
  fix_hint: 不允许硬编码模块名，使用动态注册

- id: RIDL03
  name: class id 命名不规范
  severity: medium
  check_command: |
    rg "class_id.*=.*\"[a-z]" -g '*.ridl'
  fix_hint: class id 必须 ALL_CAPS，如 GLOBAL_FOO_BAR
```

### GC 相关

```yaml
- id: GC01
  name: Global 过度使用
  severity: low
  check_command: |
    rg "Global::<" -g '*.rs'
  fix_hint: 跨 async 持有推荐使用 Root<T> 而非 Global<T>

- id: GC02
  name: GC 测试缺少 finalizer 验证
  severity: medium
  check_command: |
    rg "gc.*test" -g '*.rs' -A 10 | rg -v "finalizer|JS_SetClassFinalizer"
  fix_hint: GC 回收测试应通过 finalizer 验证对象被回收
```

## 审查上下文与例外规则

### Test-only 代码的放宽规则

**判断依据**：代码是否在 `#[cfg(test)]` 或 `tests/` 目录下。

**例外场景**：
- ✅ **unwrap 允许**：测试代码中 unwrap 可接受（失败即 panic 是预期）
- ✅ **panic! 允许**：测试断言失败可直接 panic
- ⚠️ **仍需基本逻辑**：即使测试代码也应避免明显的逻辑错误

**示例**：
```rust
// ✅ 可接受：测试代码中的 unwrap
#[test]
fn test_root() {
    let ctx = Context::new().unwrap();  // 测试失败即 panic
}

// ❌ 仍需修复：即使在测试中也应避免
#[test]
fn test_index() {
    let v = vec![1];
    assert_eq!(v[100], 1);  // 必然 panic，非预期失败
}
```

### Clippy 互补原则

本规则集与 clippy 互补，不重复：
- 本规则集关注：项目特定约定、平台陷阱、架构约束
- clippy 关注：通用 Rust 习惯用法、性能优化、常见错误模式

保留 `#[allow(clippy::...)]` 的优先级，不覆盖用户决策。

## Review 工作流建议

1. **发现问题时**，先检查是否在 `#[cfg(test)]` 或 `tests/` 下
2. **如果是测试代码**，对 unwrap/panic 降级为 INFO
3. **仍需检查**：逻辑错误、类型安全、项目特定约束（RIDL/GC）
