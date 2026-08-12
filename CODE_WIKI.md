# create-language Code Wiki

> 一门多范式（过程式 + 函数式）、渐进类型、基于 GC 的编程语言前端实现。

---

## 目录

- [1. 项目概述](#1-项目概述)
- [2. 整体架构](#2-整体架构)
- [3. 目录结构](#3-目录结构)
- [4. 模块职责详解](#4-模块职责详解)
  - [4.1 lib.rs — 入口与模块导出](#41-librs--入口与模块导出)
  - [4.2 ast.rs — 抽象语法树](#42-astrs--抽象语法树)
  - [4.3 lexer.rs — 词法分析器](#43-lexerrs--词法分析器)
  - [4.4 parser.rs — 语法分析器](#44-parserrs--语法分析器)
- [5. 关键数据结构与类型](#5-关键数据结构与类型)
  - [5.1 AST 核心类型](#51-ast-核心类型)
  - [5.2 Token 与 TokenKind](#52-token-与-tokenkind)
  - [5.3 错误类型](#53-错误类型)
- [6. 依赖关系](#6-依赖关系)
- [7. 运行方式](#7-运行方式)
- [8. 语法速览](#8-语法速览)
- [9. 已实现特性清单](#9-已实现特性清单)
- [10. 后续规划](#10-后续规划)

---

## 1. 项目概述

**create-language** 是一个从零构建的编程语言前端项目，目标是实现一门：

- **多范式**：同时支持过程式（`fun`/`val`/`var`）和函数式（Lambda、高阶函数）
- **渐进类型**：类型标注可选，类型推断与静态检查逐步增强
- **基于 GC**：自动内存管理，免除手动释放
- **双后端**：解释执行（字节码 VM）+ 编译执行（LLVM）

当前阶段完成了前端三件套：**词法分析 → 语法分析 → 抽象语法树（AST）**。

### 技术栈

| 维度 | 选择 |
|------|------|
| 语言 | Rust (Edition 2021) |
| 构建工具 | Cargo |
| 外部依赖 | 零（仅使用 `std`） |
| 代码行数 | ~1400 行 |

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    Source Code (.cl)                     │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                   Lexer (lexer.rs)                       │
│  · 字符流 → Token 流                                     │
│  · 关键字识别、字符串/数字/注释解析                       │
│  · 错误行号与列号定位                                     │
└────────────────────────┬────────────────────────────────┘
                         │  Vec<Token>
                         ▼
┌─────────────────────────────────────────────────────────┐
│                  Parser (parser.rs)                      │
│  · Token 流 → AST（递归下降解析器）                       │
│  · 运算符优先级（ Pratt Parser 风格）                     │
│  · 完整的语法错误诊断                                     │
└────────────────────────┬────────────────────────────────┘
                         │  Program (AST)
                         ▼
┌─────────────────────────────────────────────────────────┐
│                  AST (ast.rs)                            │
│  · 不可变数据结构（derive Clone/PartialEq）               │
│  · 表达式、语句、声明、类型系统全覆盖                     │
│  · 可直接序列化/遍历，供后端使用                         │
└─────────────────────────────────────────────────────────┘
```

### 数据流

```
源代码字符串 ──Lexer──▶ Token 向量 ──Parser──▶ Program (AST)
```

这是一个经典的编译器前端管线：**扫描 → 解析 → 建树**。

---

## 3. 目录结构

```
create-language/
├── Cargo.toml                  # Cargo 项目配置（零外部依赖）
├── spec.md                     # 项目规格说明文档
├── grammar.ebnf                # EBNF 文法定义
├── docs/
│   └── source-map/
│       └── frontend.md         # 前端模块映射 & 实现状态
└── src/
    ├── lib.rs                  # crate 入口，导出 ast / lexer / parser 模块
    ├── ast.rs                  # 抽象语法树（~476 行）
    ├── lexer.rs                # 词法分析器（~737 行）
    └── parser.rs               # 语法分析器（~1426 行）
```

---

## 4. 模块职责详解

### 4.1 lib.rs — 入口与模块导出

[lib.rs](file:///workspace/create-language/src/lib.rs)

```rust
pub mod ast;
pub mod lexer;
pub mod parser;
```

作为 crate 入口，仅负责声明并公开三个子模块。外部通过 `create_language::lexer::Lexer`、`create_language::parser::Parser`、`create_language::ast::*` 访问各组件。

### 4.2 ast.rs — 抽象语法树

[ast.rs](file:///workspace/create-language/src/ast.rs)

**职责**：定义源代码的完整树状结构。每个语法构造对应一个 Rust 枚举或结构体。

**设计原则**：
- 所有类型 `#[derive(Debug, Clone, PartialEq)]`，便于测试比较和 AST 变换
- 使用 `type Identifier = String` 统一标识符表示
- 表达式与语句分离：`Expr`（可产生值）与 `Stmt`（副作用导向）
- 类型系统内嵌 AST：`Type` 枚举贯穿函数签名、变量声明、泛型参数

**核心组成**：

| 类别 | 关键类型 | 说明 |
|------|----------|------|
| 顶层结构 | `Program`, `TopLevel` | 整个文件的 AST 根节点 |
| 声明 | `FunctionDecl`, `StructDecl`, `ClassDecl`, `EnumDecl`, `TraitDecl`, `ImplDecl`, `DataClassDecl` | 全部类型/函数声明 |
| 语句 | `Stmt`, `Block`, `VarDecl`, `Assign` | 控制流与声明 |
| 表达式 | `Expr`, `BinaryExpr`, `CallExpr`, `LambdaExpr` 等 | 所有表达式节点 |
| 类型 | `Type` | 完整类型系统（命名/元组/数组/函数/结果/选项/联合/可空） |
| 模式 | `Pattern` | match 表达式的模式匹配 |
| 字面量 | `Literal` | Int/Float/String/Char/Bool/Null |

**`Type` 的 Display 实现**（[ast.rs#L423-L476](file:///workspace/create-language/src/ast.rs#L423-L476)）提供了人类可读的类型格式化，方便调试与错误信息生成。

### 4.3 lexer.rs — 词法分析器

[lexer.rs](file:///workspace/create-language/src/lexer.rs)

**职责**：将源代码字符串转换为 Token 序列。

**关键结构**：

| 类型 | 行号 | 说明 |
|------|------|------|
| `Token` | [lexer.rs#L4-L9](file:///workspace/create-language/src/lexer.rs#L4-L9) | 词法单元，包含 `kind`、`lexeme`、`line`、`column` |
| `TokenKind` | [lexer.rs#L23-L115](file:///workspace/create-language/src/lexer.rs#L23-L115) | 80+ 种 Token 类型的枚举 |
| `LexerError` | [lexer.rs#L206-L218](file:///workspace/create-language/src/lexer.rs#L206-L218) | 词法错误，含精确位置 |
| `Lexer` | [lexer.rs#L220-L229](file:///workspace/create-language/src/lexer.rs#L220-L229) | 词法分析器本体 |

**`TokenKind` 分类**：

- **关键字**（38 种）：`package`, `import`, `fun`, `async`, `struct`, `class`, `enum`, `trait`, `impl`, `if`, `else`, `match`, `while`, `until`, `for`, `return`, `break`, `continue`, `throw`, `try`, `catch`, `finally`, `spawn`, `receive` 等
- **字面量**（4 种）：`Int(i64)`, `Float(f64)`, `String(String)`, `Char(char)`
- **标识符**：`Ident(String)`
- **定界符**（9 种）：`(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`
- **运算符**（30+ 种）：算术、比较、逻辑、赋值、范围等
- **特殊**：`Eof`

**核心方法**：

| 方法 | 行号 | 说明 |
|------|------|------|
| `Lexer::new(source)` | [lexer.rs#L232-L245](file:///workspace/create-language/src/lexer.rs#L232-L245) | 创建词法分析器实例 |
| `lex(&mut self) -> Result<Vec<Token>>` | [lexer.rs#L247-L260](file:///workspace/create-language/src/lexer.rs#L247-L260) | 主入口，扫描完整源代码 |
| `next_token(&mut self)` | [lexer.rs#L262-L374](file:///workspace/create-language/src/lexer.rs#L262-L374) | 核心扫描逻辑，按字符分派 |
| `skip_whitespace_and_comments(&mut self)` | [lexer.rs#L422-L435](file:///workspace/create-language/src/lexer.rs#L422-L435) | 跳过空白和注释（支持 `//` 和 `/* */` 嵌套） |
| `string(&mut self)` | [lexer.rs#L477-L508](file:///workspace/create-language/src/lexer.rs#L477-L508) | 字符串解析，支持转义序列 |
| `number(&mut self)` | [lexer.rs#L543-L557](file:///workspace/create-language/src/lexer.rs#L543-L557) | 数字入口，自动识别十进制/十六进制/二进制/浮点 |
| `identifier_or_keyword(&mut self)` | [lexer.rs#L638-L687](file:///workspace/create-language/src/lexer.rs#L638-L687) | 标识符与关键字的区分 |

**特性亮点**：
- 支持 **嵌套块注释**（`/* /* */ */`），深度跟踪
- 支持 **转义序列**：`\n`, `\t`, `\r`, `\\`, `\"`, `\'`
- 支持 **多种数字格式**：十进制 `42`、十六进制 `0xFF`、二进制 `0b1010`、浮点 `3.14`、科学计数法 `1.0e10`
- 行号列号精确追踪，错误信息友好

### 4.4 parser.rs — 语法分析器

[parser.rs](file:///workspace/create-language/src/parser.rs)

**职责**：将 Token 流解析为 AST。采用**递归下降解析器**模式。

**关键结构**：

| 类型 | 行号 | 说明 |
|------|------|------|
| `ParserError` | [parser.rs#L5-L18](file:///workspace/create-language/src/parser.rs#L5-L18) | 语法错误类型 |
| `Parser` | [parser.rs#L20-L23](file:///workspace/create-language/src/parser.rs#L20-L23) | 语法分析器本体，持有所需 Token 序列和当前位置 |

**核心方法**：

| 方法 | 行号 | 说明 |
|------|------|------|
| `parse(&mut self) -> Result<Program>` | [parser.rs#L30-L52](file:///workspace/create-language/src/parser.rs#L30-L52) | 主入口，解析完整程序 |
| `parse_top_level(&mut self)` | [parser.rs#L81-L94](file:///workspace/create-language/src/parser.rs#L81-L94) | 分派顶层声明类型 |
| `parse_function_decl(&mut self)` | [parser.rs#L96-L118](file:///workspace/create-language/src/parser.rs#L96-L118) | 解析函数声明 |
| `parse_struct_decl(&mut self)` | [parser.rs#L173-L185](file:///workspace/create-language/src/parser.rs#L173-L185) | 解析结构体声明 |
| `parse_class_decl(&mut self)` | [parser.rs#L260-L281](file:///workspace/create-language/src/parser.rs#L260-L281) | 解析类声明 |
| `parse_enum_decl(&mut self)` | [parser.rs#L325-L344](file:///workspace/create-language/src/parser.rs#L325-L344) | 解析枚举声明 |
| `parse_trait_decl(&mut self)` | [parser.rs#L371-L386](file:///workspace/create-language/src/parser.rs#L371-L386) | 解析 Trait 声明 |
| `parse_impl_decl(&mut self)` | [parser.rs#L418-L437](file:///workspace/create-language/src/parser.rs#L418-L437) | 解析 impl 声明 |
| `parse_stmt(&mut self)` | [parser.rs#L439-L492](file:///workspace/create-language/src/parser.rs#L439-L492) | 解析语句 |
| `parse_expr(&mut self)` | [parser.rs#L711-L713](file:///workspace/create-language/src/parser.rs#L711-L713) | 表达式入口 |

**表达式优先级解析链**（[parser.rs#L715-L921](file:///workspace/create-language/src/parser.rs#L715-L921)）：

```
parse_expr → parse_or_expr
  → parse_and_expr
    → parse_equality_expr
      → parse_relational_expr
        → parse_range_expr
          → parse_additive_expr
            → parse_multiplicative_expr
              → parse_unary_expr
                → parse_await_expr
                  → parse_postfix_expr
                    → parse_primary_expr
```

这是标准的优先级攀爬模式，从最低优先级（`||`）到最高（primary）。

**辅助方法**：

| 方法 | 行号 | 说明 |
|------|------|------|
| `check`, `match_token`, `advance` | [parser.rs#L1305-L1327](file:///workspace/create-language/src/parser.rs#L1305-L1327) | Token 流位置操作 |
| `peek_kind`, `peek_ahead_kind` | [parser.rs#L1329-L1335](file:///workspace/create-language/src/parser.rs#L1329-L1335) | 前瞻操作 |
| `looks_like_lambda` | [parser.rs#L1088-L1115](file:///workspace/create-language/src/parser.rs#L1088-L1115) | Lambda 表达式预判 |
| `parse_type` 系列 | [parser.rs#L1138-L1227](file:///workspace/create-language/src/parser.rs#L1138-L1227) | 类型解析（联合/可空/命名/元组/数组/函数） |

---

## 5. 关键数据结构与类型

### 5.1 AST 核心类型

#### Program — AST 根节点

```rust
pub struct Program {
    pub package: Option<PackageDecl>,   // 可选的包声明
    pub imports: Vec<ImportStmt>,       // 导入语句列表
    pub items: Vec<TopLevel>,           // 顶层声明列表
}
```

#### TopLevel — 顶层声明枚举

```rust
pub enum TopLevel {
    Function(FunctionDecl),
    Struct(StructDecl),
    DataClass(DataClassDecl),
    Class(ClassDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Stmt(Stmt),
}
```

#### Expr — 表达式枚举（21 个变体）

```rust
pub enum Expr {
    Await(Box<Expr>),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    Identifier(Identifier),
    Literal(Literal),
    Grouping(Box<Expr>),
    Block(Block),
    If(Box<IfStmt>),
    Match(Box<MatchStmt>),
    Lambda(LambdaExpr),
    StructLiteral(StructLiteralExpr),
    DataClassLiteral(DataClassLiteralExpr),
    ArrayLiteral(Vec<Expr>),
    Spawn(Box<Expr>),
    Receive(Option<Box<Expr>>),
    Nullable(Box<Expr>),     // ?. 安全调用
    NonNull(Box<Expr>),      // !! 非空断言
}
```

#### Type — 类型系统

```rust
pub enum Type {
    Named(Identifier, Vec<Type>),           // 命名类型 + 泛型参数
    Tuple(Vec<Type>),                       // 元组类型
    Array(Box<Type>),                       // 数组类型
    Func(Vec<Type>, Option<Box<Type>>),     // 函数类型
    Result(Box<Type>, Box<Type>>),          // Result<T, E>
    Option(Box<Type>),                      // Option<T>
    Union(Vec<Type>),                       // 联合类型 T | U
    Nullable(Box<Type>),                    // 可空类型 T?
}
```

#### Stmt — 语句枚举

```rust
pub enum Stmt {
    VarDecl(VarDecl),
    Assign(Assign),
    Expr(Expr),
    If(IfStmt),
    Match(MatchStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Until(UntilStmt),
    For(ForStmt),
    ForIn(ForInStmt),
    Return(Option<Expr>),
    Break,
    Continue,
    Throw(Expr),
    Try(TryStmt),
    Block(Block),
}
```

### 5.2 Token 与 TokenKind

`Token` 结构体包含四个字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `kind` | `TokenKind` | Token 类型枚举 |
| `lexeme` | `String` | 原始文本片段 |
| `line` | `usize` | 行号（1-based） |
| `column` | `usize` | 列号（1-based） |

`TokenKind` 共 ~80 个变体，分为 **关键字、字面量、标识符、定界符、运算符、特殊** 六大类。

### 5.3 错误类型

| 类型 | 文件 | 说明 |
|------|------|------|
| `LexerError` | [lexer.rs#L206-L218](file:///workspace/create-language/src/lexer.rs#L206-L218) | 词法错误，含 `message`、`line`、`column` |
| `ParserError` | [parser.rs#L5-L18](file:///workspace/create-language/src/parser.rs#L5-L18) | 语法错误，含 `message`、`line`、`column` |

两者均实现了 `Display` 和 `std::error::Error` trait。

---

## 6. 依赖关系

### 外部依赖

**零外部依赖**。项目仅使用 Rust 标准库：

```toml
[dependencies]
# 空
```

| 依赖 | 用途 |
|------|------|
| `std::fmt` | 实现 `Display` trait（TokenKind、Type、错误类型） |
| `std::str::Chars` | 词法分析器的字符迭代 |

### 内部模块依赖

```
lib.rs
  ├── ast.rs        (无内部依赖，基础数据结构层)
  ├── lexer.rs      (依赖 ast.rs：无直接引用，仅定义 Token 层)
  └── parser.rs     (依赖 ast.rs + lexer.rs：引用 AST 类型和 Token 类型)
```

依赖方向清晰：**parser → ast + lexer**，**lexer → 独立**，**ast → 独立**。

### 典型调用链

```rust
// 1. 创建 Lexer
let mut lexer = Lexer::new(source_code);

// 2. 扫描为 Token
let tokens = lexer.lex()?;

// 3. 创建 Parser
let mut parser = Parser::new(tokens);

// 4. 解析为 AST
let program = parser.parse()?;
```

---

## 7. 运行方式

### 环境要求

- **Rust** >= 1.56 (Edition 2021)
- **Cargo**（随 Rust 安装）

### 常用命令

```bash
# 进入项目目录
cd create-language/

# 构建（开发模式）
cargo build

# 构建（发布模式，优化）
cargo build --release

# 运行所有测试
cargo test

# 检查代码（快速编译检查，不生成二进制）
cargo check

# 运行测试并查看详细输出
cargo test -- --nocapture
```

### 测试覆盖

当前共 **6 个单元测试**，全部通过：

| 测试 | 位置 | 内容 |
|------|------|------|
| `lexer::tests::lex_keywords_and_symbols` | [lexer.rs#L710-L718](file:///workspace/create-language/src/lexer.rs#L710-L718) | 验证关键字和符号识别 |
| `lexer::tests::lex_string_and_comments` | [lexer.rs#L720-L726](file:///workspace/create-language/src/lexer.rs#L720-L726) | 验证字符串和注释跳过 |
| `lexer::tests::lex_numbers` | [lexer.rs#L728-L736](file:///workspace/create-language/src/lexer.rs#L728-L736) | 验证十进制/浮点/十六进制/二进制 |
| `parser::tests::parse_hello_function` | [parser.rs#L1390-L1399](file:///workspace/create-language/src/parser.rs#L1390-L1399) | 解析简单函数 |
| `parser::tests::parse_variable_and_if` | [parser.rs#L1401-L1414](file:///workspace/create-language/src/parser.rs#L1401-L1414) | 解析变量和 if/else |
| `parser::tests::parse_lambda` | [parser.rs#L1416-L1426](file:///workspace/create-language/src/parser.rs#L1416-L1426) | 解析 Lambda 表达式 |

### 快速使用示例

```rust
use create_language::lexer::Lexer;
use create_language::parser::Parser;

fn main() {
    let source = r#"
        fun fibonacci(n: int): int {
            if (n <= 1) { return n; }
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
    "#;

    match Lexer::new(source).lex() {
        Ok(tokens) => {
            match Parser::new(tokens).parse() {
                Ok(program) => println!("{:?}", program),
                Err(e) => eprintln!("Parse error: {}", e),
            }
        }
        Err(e) => eprintln!("Lex error: {}", e),
    }
}
```

---

## 8. 语法速览

### 函数

```
fun add(a: int, b: int): int { return a + b; }
async fun fetch(url: string): string { ... }
```

### 变量

```
val x: int = 1;       // 不可变
var y: int = 2;       // 可变
```

### 控制流

```
if (cond) { ... } else if (other) { ... } else { ... }
while (cond) { ... }
while until cond { ... }    // until 条件取反
do { ... } while (cond);
for (var i = 0; i < 10; i++) { ... }
for (item in list) { ... }
match (expr) { pattern => expr, _ => default }
```

### 复合类型

```
struct Point { x: int, y: int }
data class User(val name: string, val age: int)
class Animal { val name: string; init(n: string) { name = n; } fun speak(): string { ... } }
enum Option<T> { Some(T), None }
trait Display { fun toString(): string; }
impl Display for User { fun toString(): string { ... } }
```

### Lambda

```
val f = (a: int, b: int): int -> a + b;
val g = { (x: int) -> x * 2 };
```

### 错误处理

```
try { throw "error"; } catch (e: string) { ... } finally { ... }
val result: Result<int, string> = ...;
val maybe: Option<int> = ...;
val x = maybe?.field;    // 安全调用
val y = maybe!!;          // 非空断言
```

### 并发

```
async fun task(): string { ... }
val handle = spawn task();
receive(msg)
```

### 完整的 EBNF 文法

参见项目根目录的 [grammar.ebnf](file:///workspace/grammar.ebnf) 文件，定义了从 `program` 到 `escape_sequence` 的完整语法规则。

---

## 9. 已实现特性清单

### 词法分析 (Lexer) — ✅ 已完成

- [x] 关键字识别（38 个）
- [x] 标识符与关键字区分
- [x] 整数字面量（十进制、十六进制 `0x`、二进制 `0b`）
- [x] 浮点字面量（含科学计数法）
- [x] 字符串字面量（含转义序列）
- [x] 字符字面量（含转义序列）
- [x] 布尔字面量
- [x] 注释（单行 `//`、多行 `/* */` 支持嵌套）
- [x] 运算符（算术、比较、逻辑、赋值）
- [x] 定界符

### 语法分析 (Parser) — ✅ 已完成

- [x] 包声明 `package`
- [x] 导入 `import "..." as ...`
- [x] 函数声明 `fun`（含泛型、可选参数、返回类型）
- [x] 异步函数 `async fun`
- [x] 变量声明 `val` / `var`（含类型注解和默认值）
- [x] 赋值与复合赋值（`=`, `+=`, `-=`, `*=`, `/=`, `%=`）
- [x] `if` / `else if` / `else`
- [x] `match` 表达式（含模式匹配）
- [x] `while` / `while until` / `do while`
- [x] `for` C 风格 / `for ... in`
- [x] `return` / `break` / `continue` / `throw`
- [x] `try / catch / finally`
- [x] `struct`（含泛型）
- [x] `data class`（含 `val`/`var` 参数）
- [x] `class`（含继承、字段、构造器、方法）
- [x] `enum`（含泛型、变体数据）
- [x] `trait` / `impl`（含方法签名和默认实现）
- [x] Lambda 表达式（两种语法：`(params) -> body` 和 `{ params -> body }`）
- [x] 结构体字面量
- [x] 数据类字面量
- [x] 数组字面量
- [x] 表达式完整优先级（`||` → `&&` → `==` → `<` → `..` → `+` → `*` → 一元 → 后缀）
- [x] 泛型参数与泛型实参
- [x] 联合类型 `T | U`
- [x] 可空类型 `T?`
- [x] 函数类型 `func(params): ret`
- [x] `Result<T, E>` 特殊类型
- [x] `Option<T>` 特殊类型
- [x] `spawn` / `receive` 并发原语
- [x] `?.` 安全调用运算符
- [x] `!!` 非空断言运算符
- [x] `await` 表达式

### 待实现

- [ ] 类型检查器（`type_checker.rs`）
- [ ] 语义分析（未定义变量、作用域解析）
- [ ] 宏系统
- [ ] 模块解析器（从文件路径加载 `.cl` 文件）
- [ ] 字节码 VM
- [ ] LLVM 后端

---

## 10. 后续规划

按 [spec.md](file:///workspace/create-language/spec.md) 定义的实现阶段：

| 阶段 | 内容 | 状态 |
|------|------|------|
| **阶段 1** | Lexer / Parser / AST | ✅ 完成 |
| **阶段 2** | 类型检查（渐进类型推断与检查） | 🔜 下一步 |
| **阶段 3** | 字节码 VM（指令集、栈机、GC） | ⏳ 规划中 |
| **阶段 4** | LLVM 后端（生成本地代码） | ⏳ 规划中 |
| **阶段 5** | 标准库（字符串、集合、IO、并发原语） | ⏳ 规划中 |

### Source Map

详细的模块与规格映射见 [frontend.md](file:///workspace/create-language/docs/source-map/frontend.md)。

---

> **文档生成时间**：2026-08-12
> **基于代码版本**：create-language v0.1.0