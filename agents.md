# AGENTS.md — 子代理协作工作流

## 核心原则

1. **主协调者绝不碰代码**：主协调者只能读写 `.md` 文件，绝不可直接 `Edit`/`Write` 任何 `.rs`、`.toml` 等源码文件。代码编写全部由子代理（worker）完成。
2. **每完成一个里程碑立即 push**：不积攒技术债，避免沙箱重置丢失进度。
3. **高自主决策**：仅在重大架构决策时询问用户，日常实现细节自主判断。

## 子代理类型

| 角色 | 职责 |
|------|------|
| **worker** | 编写代码，严格按 Java 原始实现 1:1 移植，禁止偷懒（空实现/TODO/占位除非原代码有） |
| **verifier** | 严格验收 worker 的代码，对比 Java 源码，找出所有方法签名差异、语义偏离、遗漏 |
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

## 里程碑节奏

- 每完成 **3 个待办项** 进行一次集成验收
- 前 3 个任务仅 `cargo test`，后续尝试 `cargo test --workspace`
- 验收通过后立即 `git push`

## 代码规范

- 所有 Rust 文件须包含 Oracle 版权头和 SPDX 许可证标识
- 保持与 Java 原代码相同的方法签名和语义
- 模块名 `JVMCI` → `RustCI`（仅顶层命名变更）
- 禁止添加无关注释，像专业程序员一样写代码

## Source Map 管理

- 主协调者维护 `docs/source-map/` 目录下的 MD 文件
- 每个 Java 类到 Rust 文件的映射需记录状态（todo/in_progress/done）
- 子代理完成任务后须回写 source map

## Git 规范

- 使用 conventional commits 格式：`feat(scope): description`
- 每个里程碑完成后立即 push，token 由主协调者持有
- 提交前确认 `cargo check --workspace` 绿色

## 目录结构

```
/workspace/create-language/
  ├── agents.md          ← 本文件
  ├── docs/source-map/   ← 类映射文档
  ├── crates/            ← Cargo workspace crates
  └── spec.md            ← 项目规格说明
```

---

> 主协调者记住：**你不写代码，你只调度子代理。写完就 push。**