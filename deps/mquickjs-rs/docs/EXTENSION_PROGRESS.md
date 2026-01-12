# RIDL与mquickjs标准库集成与构建规范

## 一、核心原则与注册机制
- RIDL用于扩展而非替换mquickjs标准库，保留原有功能，仅移除示例代码和平台相关功能
- 所有标准库功能必须通过编译时静态注册，禁止运行时动态注册
- 标准库功能需在编译mquickjs时确定，并集成到mqjs_stdlib.c中
- 使用`JSValue func(JSContext *ctx, JSValue this_val, int argc, JSValue *argv)`定义JS函数
- 使用`JSPropDef`数组定义对象属性和方法，以`JS_PROP_END`结尾
- 使用`JSClassDef`定义类结构，并通过`JS_PROP_CLASS_DEF`或`JS_CFUNC_DEF`注册到全局
- 通过build_atoms在编译时生成字节码

## 二、RIDL模块化架构设计
- 支持两种注册方式：
1. 全局注册（无module声明）：函数直接注册到global对象，适用于平台标准库（如console）
2. 模块化注册（有module声明）：语法为`module system.network@1.0`，通过`require("module.name")`获取实例
- module声明作用于整个文件，必须位于文件开头，一个文件仅允许一个module声明
- 版本号格式为`主版本号.次版本号`或`主版本号`，不允许超过两部分

## 三、require机制与模块映射
- 注入全局`require(name: string) -> object`函数实现模块化（工厂模式）
- 模块在mquickjs中注册为class，但不提供构造函数
- 必须通过`require`函数获取模块实例，确保单一入口
- 使用线程安全的静态映射表（如LazyLock<Mutex<HashMap>>）存储模块名到ClassID的映射
- 在编译时为每个模块生成唯一ClassID并预注册到映射表
- require函数根据模块名查询ClassID并创建对应对象实例
- 映射表生命周期与mquickjs实例绑定，实例销毁时同步清理资源

## 四、静态代码生成架构
- 所有RIDL定义的功能必须通过ridl-tool在编译时生成Rust和C代码
- 生成代码集成到mqjs_stdlib静态库中
- 禁止依赖运行时动态注册方案
- RIDL文件必须一次性完成代码生成，不支持运行时动态添加
- 支持分阶段处理流程：
1. 收集阶段：逐步解析并存储RIDL定义
2. 合并阶段：将所有定义合并到全局上下文
3. 生成阶段：一次性生成C/Rust绑定代码
- 模块映射表和require函数依赖静态代码生成，无法实现流式编译

## 五、模块注册表生成规范
- 模块映射表必须生成为独立Rust模块文件（如module_registry.rs）
- 文件需在mquickjs-rs编译时被引入，包含：
1. 模块名到ClassID的映射表
2. ClassID静态变量定义
3. 模块类初始化函数
4. require函数实现
5. require函数注册接口
- 确保职责分离和编译时集成能力

## 六、API与项目结构规范
- 必须使用mquickjs的FFI接口进行代码生成，禁止直接使用QuickJS的C API
- 在Rust层面通过FFI与C代码交互，确保与mquickjs架构一致
- 支持将每个RIDL功能模块作为独立cargo子工程，统一纳入mquickjs-rs workspace管理
- ridl-tool需支持跨子工程扫描并收集所有RIDL files
- 代码生成阶段对所有模块的RIDL定义进行统一处理和生成

## 七、目录结构与集成机制
```
tests/ridl_tests/
├── stdlib/           # 每个RIDL模块独立目录
│   ├── *.ridl        # RIDL接口定义
│   ├── *.rs          # Rust胶水代码和实现
│   └── *.h           # C头文件定义
└── mquickjs-rs/      # 生成文件存放处
├── mqjs_stdlib_template.c
└── mquickjs_ridl_register.h
```
- 使用`mqjs_stdlib_template.c`作为基础模板
- 通过`mquickjs_ridl_register.h`头文件包含RIDL生成的注册代码
- 使用`JS_RIDL_EXTENSIONS`宏将RIDL扩展注入全局对象数组
- 所有模块的注册信息统一合并到`mquickjs_ridl_register.h`
- 通过构建系统自动收集并集成所有模块

## 八、类型匹配要求
- 使用`JSPropDef`而非`JSCFunctionListEntry`
- 确保RIDL生成的函数定义与mquickjs API类型兼容
- 正确使用`JS_CFUNC_DEF`、`JS_PROP_DOUBLE_DEF`等宏

## 九、测试文件集成要求
- 测试文件不能作为独立文件直接使用rustc编译运行
- 必须集成到项目测试模块中，通过`cargo test`命令执行
- 确保正确链接所有依赖模块和解析路径

## 十、标准库接口注册目标
- 将mquickjs标准库中平台相关的功能接口（通过mqjs_stdlib_impl.c实现）使用RIDL注册到全局

## 十一、完整集成流程
1. **RIDL定义阶段**
- 通过`.ridl`文件定义接口（如`stdlib.ridl`）
- 支持全局注册与模块化注册（`module system.network@1.0`）

2. **代码生成阶段**
- ridl-tool解析RIDL生成C函数实现
- 生成JSPropDef数组与JSClassDef类定义
- 生成模块映射表及require函数

3. **标准库集成阶段**
- 利用`build_atoms`将定义编译为字节码
- 生成`mqjs_stdlib.h`头文件包含所有静态定义
- `mqjs_stdlib_impl.c`仅保留头文件包含

4. **Rust绑定阶段**
- 通过mquickjs-rs提供Rust交互接口
- 实现模块注册表与require函数的Rust封装

## 十二、标准库初始化机制（更新说明）
- **重要更正**：实际实现中并不存在`JS_InitModuleSTDLib`函数
- 标准库功能通过`JS_NewContext`函数的第三个参数`const JSSTDLibraryDef *stdlib_def`传入
- 在`Context::new`函数中，使用静态变量`js_stdlib`作为标准库定义
- 通过`JS_NewContext(mem_start, mem_size, &js_stdlib)`调用完成标准库初始化
- `js_stdlib`静态变量定义在生成的头文件中（如`mqjs_ridl_stdlib.h`）

# mquickjs-rs RIDL扩展开发进度

另见：`docs/API.md`（mquickjs-rs 设计与 API 说明：Context/ContextHandle、ValueRef/PinnedValue、TLS current）。

## RIDL扩展符号管理方案

### 问题描述
- mquickjs.a 静态库引用了 RIDL 接口函数（如 js_say_hello）
- 这些函数在相应的 rlib 模块中实现（如 stdlib_demo）
- 链接时找不到符号，需要显式引用以防止被优化掉
- 显式引用存在问题：
  1. 关闭 ridl-extensions feature 时符号不存在，导致编译错误
  2. 存在大量 RIDL 接口时，逐个引用过于复杂

### 解决方案
采用条件宏 + 自动生成文件的方案：

1. **ridl-tool 生成符号文件**：
   - 分析所有 RIDL 文件
   - 生成 `ridl_symbols.rs` 文件，包含所有扩展函数的引用

2. **条件宏实现**：
   ```rust
   #[cfg(feature = "ridl-extensions")]
   macro_rules! mquickjs_ridl_extensions {
       () => {
           include!("ridl_symbols.rs");
       };
   }

   #[cfg(not(feature = "ridl-extensions"))]
   macro_rules! mquickjs_ridl_extensions {
       () => {
           // 空实现
       };
   }
   ```

3. **集成方式**：
   - 在 mquickjs-rs 中将宏定义为公共宏（#[macro_export]）
   - 在使用 RIDL 扩展的项目中调用 `mquickjs_ridl_extensions!()` 宏
   - 启用 ridl-extensions feature 时展开符号引用
   - 禁用时为空展开，不影响测试

### 实施计划
1. **第一阶段**：手写 ridl_symbols.rs 文件，验证机制
2. **第二阶段**：实现 mquickjs_ridl_extensions!() 宏
3. **第三阶段**：完善 ridl-tool 自动生成符号文件

### 实施结果
✓ **第一阶段完成**：已创建手写的 [ridl_symbols.rs](file:///home/peng/workspace/mquickjs-rs-demo/deps/mquickjs-rs/ridl_symbols.rs) 文件
✓ **第二阶段完成**：已在 mquickjs-rs 中实现条件宏，并将其导出为公共接口
✓ **位置调整**：宏调用从 mquickjs-rs 内部移到使用它的项目中（如 main.rs）
✓ **验证通过**：
  - 启用 ridl-extensions feature 时，符号正确引用
  - 禁用 ridl-extensions feature 时，宏为空展开，不影响测试
  - mquickjs-rs 库测试可通过（--no-default-features）
  - 主项目在启用 feature 时可正常构建

### 优势
- ✓ 解决了测试时符号不存在的编译错误
- ✓ 提供了可扩展的符号管理机制
- ✓ 与现有 feature 控制机制无缝集成
- ✓ 为未来的自动化生成奠定基础
- ✓ 遵循了关注点分离原则，宏调用位于需要它的项目中