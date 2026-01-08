# mquickjs-rs 项目文档

## 概述

本文档集合涵盖了 mquickjs-rs 项目的各个方面，包括架构设计、RIDL 语言规范、模块化构建等。

## 文档目录

### 架构相关
- [架构概述](architecture/overview.md) - 项目整体架构说明
- [模块设计](ridl/module-design.md) - RIDL 模块的设计和实现细节

### RIDL 语言相关
- [RIDL 语法与扩展](ridl/syntax-and-extension.md) - RIDL 语言的语法定义和规范
- [标准库扩展机制](ridl/stdlib-extension-mechanism.md) - 标准库扩展的实现机制和流程
- [Rust胶水代码演进](ridl/rust-glue-evolution.md) - 从C胶水代码到Rust胶水代码的演进过程

### 模块化构建相关
- [模块化构建计划](build/modular-build-plan.md) - 模块化构建的实施计划和架构规范

### 开发指南
- [开发指南](guides/development.md) - 开发者指南，包括 RIDL 模块开发、构建流程和最佳实践

### RIDL 工具链（ridl-tool）设计/实现
- [技术选型](../deps/ridl-tool/doc/TECH_SELECTION.md)
- [语法规范](../deps/ridl-tool/doc/RIDL_GRAMMAR_SPEC.md)
- [功能开发指南](../deps/ridl-tool/doc/FEATURE_DEVELOPMENT_GUIDE.md)
- [实现对应关系指南](../deps/ridl-tool/doc/IMPLEMENTATION_GUIDE.md)
- [多文件聚合设计](../deps/ridl-tool/doc/MULTIFILE_AGGREGATION_DESIGN.md)
- [设计方案](../deps/ridl-tool/doc/RIDL_TOOL_DESIGN_PLAN.md)

### 上游引擎参考（只读）
- [MicroQuickJS README](../deps/mquickjs/README.md)
- [MicroQuickJS README-CN](../deps/mquickjs/README-CN.md)

## 主要特性

- 使用 Rust 实现内存安全的 JavaScript 引擎绑定
- 通过 RIDL (Rust Interface Definition Language) 实现 JavaScript 与 Rust 之间的接口定义
- 支持模块化构建，可扩展的功能模块
- 遵循 ES5 标准的 JavaScript 运行时
- 高性能的函数调用和类型转换

## 快速开始

1. [开发指南](guides/development.md) - 从环境搭建到模块开发的完整指南
2. [架构概述](architecture/overview.md) - 了解项目整体架构
3. [RIDL 语法与扩展](ridl/syntax-and-extension.md) - 学习 RIDL 语言的使用