---
name: roots-registry-vec-option
description: RootsRegistry 使用 Mutex<Vec<Option<JSValue>>> 而非 Slab
type: decision
created: 2026-09-03
sources: [roots.rs]
---

RootsRegistry 采用 Mutex<Vec<Option<JSValue>>> 数据结构。

**为什么：** 相比 Slab，Vec<Option<T>> 实现更简单，性能足够（root 数量通常不大，遍历开销可接受），且不需要额外依赖。Root::new 时插入返回 RootId（slot index），Drop 时置空 slot（不移除避免 index 失效）。gc_mark 遍历时用 iter().flatten() 跳过空 slot。

**何时使用：** 实现类似的"支持删除的索引池"时，Vec<Option<T>> 是简单有效的选择。仅当 root 数量极大（数万级）且频繁增删时，才需要考虑 Slab 等更复杂结构。
