// 移除 stdlib 模块中的符号引用，因为该模块当前有问题
// 只保留 stdlib_demo 模块中的符号  
use stdlib_demo::js_say_hello;

// 如果有其他模块，继续添加
// use other_module::other_function;