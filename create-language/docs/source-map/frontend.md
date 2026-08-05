# 前端 Source Map

## 模块映射

| 规格模块 | Rust 文件 | 状态 |
|----------|-----------|------|
| 词法分析 (Lexer) | `src/lexer.rs` | done |
| 抽象语法树 (AST) | `src/ast.rs` | done |
| 语法分析 (Parser) | `src/parser.rs` | done |
| 类型检查 (Type Checker) | `src/type_checker.rs` | todo |
| 语义分析 (Semantic Analyzer) | `src/semantic.rs` | todo |

## 已覆盖语法

- [x] 包声明 `package`
- [x] 导入 `import "..." as ...`
- [x] 函数声明 `fun`
- [x] 异步函数 `async fun`
- [x] 变量声明 `val` / `var`
- [x] 赋值与复合赋值
- [x] `if` / `else if` / `else`
- [x] `match` 表达式
- [x] `while` / `while until` / `do while`
- [x] `for` C 风格 / `for ... in`
- [x] `return` / `break` / `continue` / `throw`
- [x] `try / catch / finally`
- [x] `struct`
- [x] `data class`
- [x] `class` / `init`
- [x] `enum`
- [x] `trait` / `impl`
- [x] Lambda 表达式
- [x] 表达式优先级与结合性
- [x] 类型注解、泛型、可空类型、联合类型、函数类型
- [x] `spawn` / `receive`

## 待实现

- [ ] 类型检查器
- [ ] 语义分析（未定义变量、作用域等）
- [ ] 宏系统
- [ ] 模块解析器（从文件路径加载）
