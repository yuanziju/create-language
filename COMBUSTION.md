# COMBUSTION.md — 贡献指南

> 本文件供人类贡献者阅读。AI 代理工作流请参阅 [AGENTS.md](AGENTS.md)。

## 命名规范

| 类别 | 规范 | 示例 |
|------|------|------|
| 文件名 | `snake_case` | `token.rs`, `expr.rs` |
| 类型名 | `CamelCase` | `TokenKind`, `Parser` |
| 变量名 | `camelCase` | `nextRegister`, `loopStack` |
| 函数名 | `CamelCase` | `CompileExpr`, `ParseStmt` |
| 常量 | `SCREAMING_SNAKE_CASE` | `MAGIC`, `VERSION_MAJOR` |
| 枚举变体 | `CamelCase` | `TokenKind::LParen` |

## 代码风格

- 禁止无关注释。代码应自解释，只在非显而易见的逻辑处加注释。
- 禁止 emoji 出现在代码或注释中。
- 保持与 `spec.md` 一致的接口签名和语义。
- 使用 `rustfmt` 默认配置，不做自定义格式化。
- 通过 `cargo clippy -- -D warnings` 零警告。

## 测试规范

- 集成测试放在 `tests/` 目录，不在模块内部写 `#[cfg(test)]`。
- 测试函数名使用 `snake_case`（Rust 标准）。
- 每个公开 API 至少有一个测试覆盖。

## Git 规范

- 使用 Conventional Commits 格式：`feat(scope): description`
- scope 示例：`parser`, `compiler`, `vm`, `gc`, `bytecode`
- 提交前确认 `cargo check` 绿色。
- 禁止提交包含敏感信息（token、密码等）的文件。

## 目录结构

```
/workspace/create-language/
  ├── AGENTS.md           ← AI 子代理工作流
  ├── COMBUSTION.md       ← 本文件
  ├── grammar.ebnf        ← 语法定义
  ├── spec.md             ← 项目规格说明
  ├── tests/              ← 集成测试
  └── src/
      ├── lib.rs
      ├── ast.rs
      ├── token.rs
      ├── lexer.rs
      └── parser/
          ├── mod.rs
          ├── expr.rs
          ├── stmt.rs
          └── decl.rs
```

## 禁止事项

- 禁止在源码中写 TODO 注释而不跟进。
- 禁止提交无法编译的代码。
- 禁止绕过代码审查直接推送到 main 分支。
- 禁止引入不必要的依赖（crate）。