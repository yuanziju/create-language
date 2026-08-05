# create-language 项目规格

## 概述

实现一门多范式（过程式 + 函数式）、渐进类型、基于 GC 的编程语言。前端生成 AST，后端同时面向字节码 VM 与 LLVM/本地代码。

## 设计目标

1. 语法参考 Kotlin、Rust、现代 C，支持花括号块。
2. 同时支持解释执行（字节码 VM）和编译执行（LLVM）。
3. 渐进类型：类型标注可选，静态检查逐步增强。
4. 内置 GC，简化内存管理。
5. 支持异步、协程、Actor/Channel 等并发模型。

## 核心语法

### 函数

```kotlin
fun add(a: int, b: int): int {
    return a + b;
}
```

### 变量

```kotlin
val x: int = 1;   // 不可变
var y: int = 2;   // 可变
```

### 控制流

```kotlin
if (cond) {
    // then
} else if (other) {
    // else if
} else {
    // else
}

while (cond) { }
while until cond { }   // until 为软关键字，条件取反

for (i in 0..10) { }
for (item in list) { }
for (var i = 0; i < 10; i++) { }

match (expr) {
    pattern => expr,
    _ => default,
}
```

### 复合类型

```kotlin
struct Point { x: int, y: int }

data class User(val name: string, val age: int)

class Animal {
    val name: string;
    init(n: string) { name = n; }
    fun speak(): string { return "..."; }
}

enum Option<T> { Some(T), None }
```

### Lambda

```kotlin
val f = (a: int, b: int): int -> a + b;
```

### 模块

```kotlin
package com.example;
import "./utils.cl" as utils;
```

默认一个文件一个模块；多个文件可在开头声明相同 `package` 以共享命名空间。

### 错误处理

- `Result<T, E>` + `?` 传播
- `try / catch / throw` 异常
- `Option<T>` / `T?` 可空类型 + `?.` 安全调用

### 并发

- `async / await`
- `spawn` 轻量协程
- `receive` Actor 消息接收

## 目录结构

```
create-language/
├── Cargo.toml
├── spec.md
├── grammar.ebnf
├── docs/
│   └── source-map/
└── src/
    ├── lib.rs
    ├── ast.rs
    ├── lexer.rs
    └── parser.rs
```

## 实现阶段

1. **Lexer / Parser / AST**：完成（当前阶段）
2. **类型检查**：渐进类型推断与检查
3. **字节码 VM**：指令集、栈机、GC
4. **LLVM 后端**：生成本地代码
5. **标准库**：字符串、集合、IO、并发原语
