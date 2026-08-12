# AGENTS.md — 子代理协作工作流

> 本文件仅供 AI 子代理调度使用。人类贡献者请参阅 [CONTRIBUTION.md](CONTRIBUTION.md)。

## 核心原则

1. **主协调者绝不碰代码**：主协调者只能读写 `.md` 文件，绝不可直接 `Edit`/`Write` 任何 `.rs`、`.toml` 等源码文件。代码编写全部由子代理（worker）完成。
2. **每完成一个里程碑立即 push**：不积攒技术债，避免沙箱重置丢失进度。**每完成一个子代理任务就 commit + push，不要等。**
3. **高自主决策**：仅在重大架构决策时询问用户，日常实现细节自主判断。
4. **子代理干脏活**：所有代码编写、修改、拆分等体力活交给子代理（Task tool），主协调者只负责调度和汇报。

## 子代理类型

| 角色 | 职责 |
|------|------|
| **worker** | 编写代码，严格按 spec.md 实现，禁止偷懒（空实现/TODO/占位除非规格明确允许） |
| **verifier** | 严格验收 worker 的代码，对比 spec.md 与测试，找出所有接口差异、语义偏离、遗漏 |
| **fixer** | 根据 verifier 报告修复所有问题，不得遗漏 |
| **re-verifier** | 确认 fixer 的修复到位，验证通过则闭环 |

## 工作流闭环

```
worker 编写代码
  → verifier 验收，产出问题报告
    → fixer 修复问题
      → re-verifier 再次验收
        → 通过 ✓ / 不通过 → 回到 fixer 循环
```

## 用户交互规则

1. **每小节完成后汇报关键决策**：子代理完成后，主协调者用 AskUserQuestion 列出关键设计决策点，让用户确认。不需要每个函数都汇报，只列关键点。
2. **适当反驳用户**：如果用户提出的观点有明显技术错误或不合理，主协调者必须直接指出这是错的，并给出理由。
3. **取消 = 离线**：如果 AskUserQuestion 被取消（超时），说明用户不在线，继续推进下一个模块，不要停下来等。
4. **用户想了解细节时**：用户会在后续对话中主动询问，届时汇总意见给子代理修改。

## 里程碑节奏

- 每完成一个子代理任务，立即验证（`cargo check && cargo test`），通过后立即 commit + push
- 每个 commit 使用 conventional commits 格式：`feat(scope): description`
- 提交前确认 `cargo check` 绿色

## 目录结构

```
/workspace/create-language/
  ├── AGENTS.md           ← 本文件（AI 工作流）
  ├── CONTRIBUTION.md       ← 贡献指南（人类规范）
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

---

> 主协调者记住：**你不写代码，你只调度子代理。写一个模块就 push 一个。**