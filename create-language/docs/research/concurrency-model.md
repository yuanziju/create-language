# 并发模型与内存模型调研报告

> 调研日期: 2026-08-05
> 目标: 为支持 async/await、协程(spawn)、Actor(receive) 的语言运行时（Rust 实现）提供技术选型参考

---

## 目录

1. [Actor 模型 —— Erlang 消息传递范式](#1-actor-模型--erlang-消息传递范式)
2. [协程调度 —— Go GMP 工作窃取调度器](#2-协程调度--go-gmp-工作窃取调度器)
3. [异步状态机 —— Rust async/await 编译器变换](#3-异步状态机--rust-asyncawait-编译器变换)
4. [内存模型 —— Java JMM Happens-Before 规范](#4-内存模型--java-jmm-happens-before-规范)
5. [无锁队列 —— Lock-Free MPSC 实现](#5-无锁队列--lock-free-mpsc-实现)
6. [Actor GC 策略 —— 进程私有堆 vs 共享堆](#6-actor-gc-策略--进程私有堆-vs-共享堆)

---

## 1. Actor 模型 —— Erlang 消息传递范式

### 1.1 核心原则

Erlang 的并发模型基于 Actor 模型，由 Joe Armstrong 在其博士论文中确立：

- **进程强隔离**：每个进程拥有独立的堆、栈和邮箱，不共享任何状态
- **轻量级创建与销毁**：每个进程初始内存仅约 2KB，创建/销毁开销极低
- **消息传递是唯一交互方式**：进程间通过异步消息传递通信
- **进程拥有唯一标识**：通过 PID 定位进程
- **无共享资源**：不存在共享内存，从根本上消除数据竞争
- **非局部错误处理**：进程要么正常工作，要么失败

### 1.2 Erlang 进程生命周期

```
spawn(Module, Function, Args) → 创建新进程
self() → 获取当前进程 PID
Pid ! Message → 发送消息（异步、非阻塞）
receive ... end → 选择性接收消息
```

消息发送是**异步且非阻塞**的：发送者不等待接收者处理消息，将消息放入接收者邮箱后立即继续执行。

### 1.3 邮箱（Mailbox）设计

每个进程在创建时自动获得一个邮箱，邮箱由两部分组成：

1. **外部邮箱（Outer Mailbox）**：消息到达时先进入此处，需要锁保护（多个发送者并发写入）
2. **内部队列（Inner Queue）**：进程唤醒时从外部邮箱移入内部队列，仅进程自身可访问，无锁

```
[ SENDER A ] [ SENDER B ] [ SENDER C ]
      |              |              |
      +--------------+--------------+
                     |
                     v
           +------------------+
           |  OUTER MAILBOX   |  <-- 锁保护
           |  [Msg4][Msg3][Msg2] |
           +--------+---------+
                    |
              (BEAM 内部移动)
                    |
                    v
           +------------------+
           | INNER PROCESS Q  |  <-- 无锁（仅 Owner 访问）
           |  [Msg1][Msg0]    |
           +------------------+
```

特性：
- **FIFO 队列**：消息按到达顺序排列
- **选择性接收**：`receive` 支持模式匹配，可以跳过不匹配的消息（它们留在队列中等待后续匹配）
- **私有性**：任何进程无法窥探其他进程的邮箱

### 1.4 消息传递的内存模型

- **复制语义**：消息数据从发送者堆复制到接收者堆（维护"无共享"原则）
- **二进制优化**：大于 64 字节的二进制数据存储在全局堆上，邮箱中仅传递引用指针（Binary Reference），避免大块数据复制

### 1.5 Actor 模型映射

| Actor 模型概念 | Erlang 实现 |
|---|---|
| Actor | 进程 (Process) |
| 地址 | PID |
| 消息 | 任意 Erlang 项 |
| 行为 | 函数 |
| 创建 Actor | spawn |
| 发送消息 | `!` 操作符 |
| 接收消息 | `receive ... end` |

---

## 2. 协程调度 —— Go GMP 工作窃取调度器

### 2.1 线程模型选型

| 模型 | 描述 | 代表 | 优缺点 |
|---|---|---|---|
| **1:1** | 一个用户线程对应一个内核线程 | Linux pthread | 调度由内核完成，实现简单；创建/切换开销大（~1MB 栈，1-10μs 切换） |
| **N:1** | 多个用户线程映射到一个内核线程 | Python Greenlet | 轻量但无法利用多核，一个阻塞全局卡死 |
| **M:N** | M 个用户线程映射到 N 个内核线程 | **Go goroutine** | 兼顾并发度和资源利用率，调度器实现复杂 |

Go 选择了 M:N 模型，这是最难但最契合高并发场景的选择。

### 2.2 GMP 三要素

| 符号 | 含义 | 数量 |
|---|---|---|
| **G** (Goroutine) | 用户态轻量级线程。包含栈（初始 2KB，可增长）、调度上下文（gobuf: SP/PC/BP） | 可达百万 |
| **M** (Machine) | 操作系统线程的封装，实际执行 G 的代码 | ≤ runtime.NumThread() |
| **P** (Processor) | 逻辑 CPU 槽位，持有本地运行队列和资源上下文 | = GOMAXPROCS（默认 CPU 核心数） |

关键约束：
- 每个 P 维护一个容量为 256 的环形缓冲本地运行队列（runq），本地操作完全无锁
- 每个 P 有独立的内存分配缓存（mcache），避免全局锁竞争
- M 必须绑定一个 P 才能执行 G 的代码

### 2.3 调度循环（schedule() 函数）

```
schedule() {
    gp = findRunnable()  // 阻塞直到找到可运行的 G
    execute(gp)          // 切换到 G 的上下文，开始执行
}
```

findRunnable() 的查找顺序（精巧的层级设计）：

1. **本地队列（最高优先级）**：检查当前 P 的 runq → 完全无锁，最高频命中路径
2. **全局队列**：每调度 61 次，强制从全局队列取一个 G → 防止全局队列饥饿
3. **网络轮询器（Netpoller）**：检查 epoll/kqueue 中是否有就绪的 G
4. **工作窃取（Work Stealing）**：从其他 P 的队列尾部窃取一半的 G

### 2.4 工作窃取算法

```
1. 随机选择一个起始 P（避免惊群效应）
2. 从起始 P 开始轮询每个 P：
   a. 尝试窃取 runnext（最高优先级 G）
   b. 尝试窃取 P 的 runq 的一半
3. 检查所有 P 的定时器堆（过期定时器）
4. 如果什么都没找到 → 放弃，park M
```

窃取从尾部（runqtail）取，本地消费从头部（runqhead）取，避免竞争。

```
P0: [G1][G2][G3][G4] ← runqhead 端取
P1: [G5][G6]         ← 本地空，发起 steal
                         ↓
P0: [G1][G2]          ← 被偷走一半 (G3, G4)
P1: [G5][G6][G3][G4]
```

### 2.5 抢占式调度

- Go 使用**协作式 + 异步抢占**混合模型
- 每 2000 reductions（约函数调用次数）为一个时间片
- 在安全点（函数序言、循环回边）检查抢占标志
- Go 1.14+ 支持基于信号的异步抢占，处理长时间运行的非抢占循环

### 2.6 系统调用处理

- 执行系统调用时，M 与 P 解绑，P 被转移给其他 M 或新建的 M
- 系统调用返回后，G 尝试重新获取一个 P；若无空闲 P，G 进入全局队列
- 自旋 M 的数量限制在 GOMAXPROCS/2 以内，避免无意义空转

### 2.7 性能特征

| 属性 | OS 线程 | Goroutine |
|---|---|---|
| 栈大小 | ~1-8 MB（固定） | ~2-8 KB（可增长） |
| 创建开销 | ~1-10 μs | ~0.3 μs |
| 上下文切换 | ~1-5 μs（内核态） | ~100-200 ns（用户态） |
| 10k 并发内存 | ~10-80 GB | ~20-80 MB |

---

## 3. 异步状态机 —— Rust async/await 编译器变换

### 3.1 设计哲学

Rust 的 async/await 采用**零运行时开销**模型：
- 没有内置事件循环，没有隐式任务调度器
- `async` 关键字仅生成一个实现 `Future` trait 的状态机
- 运行时（tokio、async-std 等）由用户选择，实现完全解耦

### 3.2 Future Trait 核心

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

`poll` 方法被执行器反复调用，直到返回 `Poll::Ready`。每个 `.await` 点都是一个潜在的暂停点。

### 3.3 状态机生成机制

编译器将 `async fn` 转换为枚举型状态机：

```rust
// 源代码
async fn fetch_two_pages() -> String {
    let page1 = http_get("https://example.com/a").await;
    let page2 = http_get("https://example.com/b").await;
    format!("{page1}\n{page2}")
}

// 编译器生成的等价结构
enum FetchTwoPagesStateMachine {
    Start,
    WaitingPage1 { fut1: HttpGetFuture },
    WaitingPage2 { page1: String, fut2: HttpGetFuture },
    Complete,
}

impl Future for FetchTwoPagesStateMachine {
    type Output = String;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        loop {
            match self.as_mut().get_mut() {
                Self::Start => {
                    let fut1 = http_get("https://example.com/a");
                    *self = Self::WaitingPage1 { fut1 };
                }
                Self::WaitingPage1 { fut1 } => {
                    let page1 = match Pin::new(fut1).poll(cx) {
                        Poll::Ready(v) => v,
                        Poll::Pending => return Poll::Pending,
                    };
                    let fut2 = http_get("https://example.com/b");
                    *self = Self::WaitingPage2 { page1, fut2 };
                }
                Self::WaitingPage2 { page1, fut2 } => {
                    let page2 = match Pin::new(fut2).poll(cx) {
                        Poll::Ready(v) => v,
                        Poll::Pending => return Poll::Pending,
                    };
                    let result = format!("{page1}\n{page2}");
                    *self = Self::Complete;
                    return Poll::Ready(result);
                }
                Self::Complete => panic!("polled after completion"),
            }
        }
    }
}
```

关键转换规则：
- 每个 `.await` 点将函数分割成一个新状态
- 跨越 `.await` 的局部变量存储在状态机枚举的字段中
- 状态机的总大小 = 所有变体中最大者的大小 + 判别式
- 编译器自动在状态转换时插入 drop 调用

### 3.4 Pin 与自引用结构

当 async 函数内部的变量被借用且借用跨越了 `.await` 点时，生成的状态机包含自引用（指向自身其他字段）。如果 Future 在内存中移动，引用会失效。

- `Pin<&mut T>` 保证被固定的值不会在内存中移动（除非实现 `Unpin`）
- 编译器自动将可能产生自引用的 Future 标记为 `!Unpin`
- 大多数简单 Future 是 `Unpin` 的（只包含原始类型或 `Box` 指针）

### 3.5 Waker 唤醒机制

- `Context` 参数包含 `Waker`，是 Rust 异步运行时的核心抽象
- 当 Future 返回 `Poll::Pending` 时，必须注册 Waker
- 条件满足时，Waker 通知执行器重新 poll
- 这避免了轮询，实现了高效的事件驱动唤醒

### 3.6 性能考量

- **零开销**：状态机是栈分配的枚举，无堆分配、无 GC、无 boxing（除非显式使用 `Box::pin()`）
- **大小优化**：编译器在状态转换时自动 drop 不再需要的值，减少内存占用
- **陷阱**：在 async 函数中分配大数组会膨胀 Future 大小（如 `[u8; 1_000_000]` → 1MB Future），应使用 `Vec<u8>` 或 `Box::pin()` 替代

---

## 4. 内存模型 —— Java JMM Happens-Before 规范

### 4.1 为什么需要内存模型

现代 CPU 为提升性能做两件事：
1. **每核心有私有高速缓存（L1/L2/L3 Cache）**：写操作先写入私有缓存，延迟传播到主内存
2. **编译器和处理器会重排序指令**：在单线程视角下保持正确，但多线程下可见

这导致：**线程对共享变量的修改，其他线程可能看不到**。

### 4.2 JMM 核心抽象

```
线程A ──[读/写]──→ 工作内存A ──[load/store]──→ 主内存
线程B ──[读/写]──→ 工作内存B ──[load/store]──→ 主内存
```

- **主内存**：所有线程共享，存储实例字段、静态字段等
- **工作内存**：每个线程私有，保存主内存变量的副本
- 线程 A 修改工作内存后不会立即同步到主内存，线程 B 也无法自动感知

### 4.3 8 大 Happens-Before 规则

Happens-Before 是 JMM 定义的可见性保证规则。如果操作 A happens-before 操作 B，则 A 的结果对 B **一定可见**。

| 规则 | 说明 | 示例 |
|---|---|---|
| **1. 程序顺序规则** | 同一线程内，前面的操作 hb 后面的操作 | `a=1; b=2;` → a=1 hb b=2 |
| **2. 监视器锁规则** | unlock hb 后续对同一锁的 lock | synchronized 块之间 |
| **3. volatile 变量规则** | 对 volatile 的写 hb 后续对该变量的读 | `volatile flag;` 写后读可见 |
| **4. 线程启动规则** | Thread.start() hb 线程内任意操作 | 主线程 start() 后子线程可见其之前变量 |
| **5. 线程终止规则** | 线程内所有操作 hb 其他线程检测到终止（如 join() 返回） | t.join() 后可看到 t 中所有修改 |
| **6. 中断规则** | interrupt() hb 被中断线程检测到中断 | — |
| **7. final 域规则** | 构造函数中 final 字段的写 hb finalize() | 正确构造的对象 final 字段对所有线程可见 |
| **8. 传递性** | A hb B, B hb C ⇒ A hb C | — |

### 4.4 volatile 关键字的双重语义

1. **保证可见性**：写 volatile 时强制刷新工作内存到主内存；读 volatile 时强制从主内存加载
2. **禁止指令重排序**：编译器/CPU 不会对 volatile 读写进行重排序（插入内存屏障）

### 4.5 DRF（Data Race Free）保证

JMM 的核心保证：
- 如果程序是**无数据竞争**的（所有共享变量访问都有适当的同步），JMM 保证**顺序一致性（Sequential Consistency）**
- 数据竞争定义：两个操作访问同一内存位置，至少一个为写操作，且操作之间没有 happens-before 关系

### 4.6 对语言设计的启示

- 需要明确声明**共享变量**和**同步原语**的语义
- 需要定义 happens-before 关系以建立跨 Actor/协程的可见性保证
- Actor 模型中，消息发送天然建立 happens-before 关系（发送 hb 接收）
- 在提供共享内存机制时，需要类似 volatile 的可见性保证原语

---

## 5. 无锁队列 —— Lock-Free MPSC 实现

### 5.1 核心概念

**无锁（Lock-Free）**：系统中至少有一个线程能在有限步骤内完成操作，即使其他线程被挂起。使用 CAS（Compare-And-Swap）原语而非互斥锁。

### 5.2 SPSC（Single Producer Single Consumer）

最简单的无锁队列场景：

```
struct SpscQueue<T> {
    buffer: Box<[MaybeUninit<T>]>,
    head: AtomicUsize,   // Producer 写入
    tail: AtomicUsize,   // Consumer 写入
}
```

- **环形缓冲区（Ring Buffer）**：head 和 tail 指针在数组中循环
- **无 CAS**：仅需 atomic load/store，Producer 和 Consumer 各自独占一个指针
- **内存排序**：Producer 使用 `Release` 发布数据，Consumer 使用 `Acquire` 读取数据
- **性能**：20-40 ns/op
- **缓存行填充**：head 和 tail 之间添加 64 字节 padding，避免 false sharing

### 5.3 MPSC（Multi Producer Single Consumer）

多生产者需要竞争写入位置：

- **CAS 槽位预留**：每个 Producer 使用 CAS 预留一个 buffer 索引
- **两阶段写入**：先预留位置，再写入数据，最后标记为可读
- **Consumer 等待**：Consumer 检查下一个槽位是否已填充，填充后才读取
- **性能**：100-200 ns/op（CAS 竞争开销）
- **算法来源**：Michael-Scott 无锁队列论文

### 5.4 Rust 生态核心库

#### crossbeam

Rust 并发编程的事实标准库，提供：

| 模块 | 功能 |
|---|---|
| `crossbeam-channel` | 高性能 MPMC channel，支持 select! 宏 |
| `crossbeam-queue` | `ArrayQueue`（有界 MPMC）、`SegQueue`（无界 MPMC） |
| `crossbeam-deque` | 无锁双端队列，用于 work-stealing |
| `crossbeam-epoch` | 基于 epoch 的无锁垃圾回收 |
| `crossbeam-utils` | `CachePadded`（防 false sharing）、scoped threads |

#### crossfire

基于 crossbeam 改进的高性能锁-free channel：

- 支持 SPSC / MPSC / MPMC
- 同时支持 async 和 blocking 上下文
- v3.0 性能：Bounded SPSC +70%、MPSC +30%
- 使用 `crossbeam-queue` 的修改版，消除 crossbeam-channel 依赖

### 5.5 设计要点总结

1. **缓存行对齐**：使用 `CachePadded` 包装关键字段，避免 false sharing
2. **内存排序**：生产者用 `Release`，消费者用 `Acquire`，最小化同步开销
3. **回退策略**：CAS 失败时使用 spin/yield/park 组合，避免忙等浪费 CPU
4. **有界 vs 无界**：有界使用环形缓冲区（更快），无界使用链式分段（更灵活）
5. **async 支持**：集成 Waker 机制，在通道空/满时注册唤醒

---

## 6. Actor GC 策略 —— 进程私有堆 vs 共享堆

### 6.1 私有堆架构（Erlang/OTP 当前方案）

每个 Erlang 进程拥有独立的堆和栈，二者在同一内存块中相向增长。BEAM 使用**分代半空间复制收集器**（Cheney 算法）：

```
┌─────────────────────────────────────────────┐
│      进程 A         │      进程 B          │
│  ┌──────┬──────┐   │  ┌──────┬──────┐    │
│  │ Stack │ Heap │   │  │ Stack │ Heap │    │
│  │   ↓   │  ↓   │   │  │   ↓   │  ↓   │    │
│  └───────┴──────┘   │  └───────┴──────┘    │
└─────────────────────────────────────────────┘
```

**优点**：
- **GC 暂停局部化**：只暂停正在 GC 的进程，其他进程继续运行
- **进程终止即回收**：进程结束时整个堆 O(1) 释放，无需 GC 周期
- **高缓存局部性**：进程数据集中在连续内存区域
- **低 GC 开销**：每个堆较小，GC 扫描快

**缺点**：
- **消息传递需复制**：消息从发送者堆复制到接收者堆，O(n) 操作
- **内存碎片**：进程堆之间无法共享空闲空间
- **数据重复**：如果多个进程持有相同数据，存在多份拷贝

**二进制优化**：大于 64 字节的二进制数据存储在全局堆上（引用计数），进程间仅传递引用指针。

### 6.2 共享堆架构（ETOS 方案）

所有进程共享一个统一堆，每个进程保留自己的栈：

```
┌─────────────────────────────────────────────┐
│        进程 A      │      进程 B           │
│     ┌──────┐      │     ┌──────┐          │
│     │ Stack │      │     │ Stack │          │
│     └──────┘      │     └──────┘          │
│         │         │         │              │
│         └────┬────┘         │              │
│              └──────────────┘              │
│                    ↓                        │
│        ┌───────────────────┐               │
│        │   统一共享堆       │               │
│        └───────────────────┘               │
└─────────────────────────────────────────────┘
```

**优点**：
- **快速消息传递**：发送消息仅需复制引用到接收者邮箱，O(1) 操作
- **低内存占用**：无数据重复，所有进程共享同一对象
- **低碎片化**：整个内存可被任何进程使用

**缺点**：
- **GC 暂停全局化**：需要扫描整个堆，根集大
- **GC 频率高**：任何进程分配失败都触发全局 GC
- **实现复杂**：需要处理跨进程引用

### 6.3 混合架构（已废弃）

Erlang/OTP R8 曾尝试混合架构：每个进程有私有堆 + 全局共享堆存放消息数据。由于锁竞争和 GC 缓慢，最终被废弃。

### 6.4 对比总结

| 维度 | 私有堆 | 共享堆 |
|---|---|---|
| 消息传递 | 复制数据，O(n) | 引用传递，O(1) |
| GC 暂停 | 局部，仅当前进程 | 全局，所有进程 |
| 内存效率 | 可能有重复数据 | 无重复，更高效 |
| 缓存局部性 | 高 | 低 |
| 进程终止 | O(1) 释放 | 需要 GC 扫描 |
| 实现复杂度 | 较低 | 较高 |

### 6.5 对语言运行时的设计启示

- 对于 Actor 并发模型，**私有堆是更务实的选择**：GC 暂停局部化对延迟敏感应用至关重要
- 采用**二进制/大对象优化**：对大数据使用全局堆 + 引用计数，避免复制开销
- 考虑**消息大小阈值**：小消息复制，大消息引用传递
- 在 Rust 实现中，可以利用所有权系统天然的"无共享"特性简化 GC

---

## 附录：关键参考资料

- Erlang 官方文档: [Erlang Garbage Collector](https://erlang.org/documentation/doc-12.0/erts-12.0/doc/html/GarbageCollection.html)
- Go 调度器设计文档: [Go Scheduler Design](https://golang.org/pkg/runtime/#Scheduler)
- Rust async 官方文档: [Comprehensive Rust - Async State Machine](https://google.github.io/comprehensive-rust/concurrency/async/state-machine.html)
- JLS Chapter 17: [Threads and Locks](https://docs.oracle.com/javase/specs/jls/se18/html/jls-17.html)
- Crossbeam: [GitHub - crossbeam-rs/crossbeam](https://github.com/crossbeam-rs/crossbeam)
- Feeley, M. (2001): "A Case for the Unified Heap Approach to Erlang Memory Management"
- Michael, M. & Scott, M. (1996): "Simple, Fast, and Practical Non-Blocking and Blocking Concurrent Queue Algorithms"