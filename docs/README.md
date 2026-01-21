# 项目文档（docs/）

本目录是本仓库的**唯一文档入口**。

## 推荐阅读（从现行口径开始）

- 开发/构建：
  - [开发指南](guides/development.md)
  - [构建流水线（现行口径）](build/pipeline.md)

- RIDL（现行口径 / Source of Truth）：
  - [RIDL 语义总览](ridl/overview.md)
  - [ridl_context_init(ctx)](ridl/context-init.md)
  - [require materialize 语义](ridl/require-materialize.md)
  - [聚合产物与边界](ridl/codegen-outputs.md)

- 架构：
  - [架构概述](architecture/overview.md)
  - [Profiles + Registry（历史/部分过时：Plan B）](legacy/plan-b-profiles-and-registry.md)

## 目录索引

- [RIDL](ridl/README.md)
- [Build / Tooling](build/README.md)
- [Architecture](architecture/README.md)
- [Planning（过程归档，不是规范）](planning/README.md)
- [Legacy（过时文档）](legacy/README.md)

## 外部依赖文档（只读）

这些文档位于依赖仓库中，不属于本仓库现行口径，但可作为实现参考：

- `deps/ridl-tool/doc/*`
- `deps/mquickjs/README*`