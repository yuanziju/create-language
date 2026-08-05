# 并发模型 (Threading & Concurrency Model)

## 三层并发抽象

```
┌─────────────────────────────────────────────────────┐
│  Layer 3: Actor 模型 (高级并发)                       │
│  ┌────────┐  ┌────────┐  ┌────────┐                │
│  │ Actor A │  │ Actor B │  │ Actor C │   ...          │
│  │ pid=1   │  │ pid=2   │  │ pid=3   │                │
│  │ 私有堆  │  │ 私有堆  │  │ 私有堆  │                │
│  │ 邮箱    │  │ 邮箱    │  │ 邮箱    │                │
│  └────────┘  └────────┘  └────────┘                │
│       │           │           │                      │
│       └───────────┼───────────┘                      │
│                   │                                  │
│            消息传递 + 无共享                          │
├─────────────────────────────────────────────────────┤
│  Layer 2: 协程调度 (M:N Green Threads)               │
│  ┌──────┐  ┌──────┐  ┌──────┐                      │
│  │  P0   │  │  P1   │  │  P2   │  (Processors)     │
│  │ ┌──┐ │  │ ┌──┐ │  │ ┌──┐ │                      │
│  │ │G │ │  │ │G │ │  │ │G │ │  (Goroutines)       │
│  │ │G │ │  │ │G │ │  │ │G │ │                      │
│  │ │G │ │  │ │G │ │  │ │G │ │                      │
│  │ └──┘ │  │ └──┘ │  │ └──┘ │                      │
│  └──────┘  └──────┘  └──────┘                      │
│       │         │         │                          │
│  ┌──────────────────────────────────┐               │
│  │     Global Run Queue             │               │
│  │  (work-stealing 后备)            │               │
│  └──────────────────────────────────┘               │
├─────────────────────────────────────────────────────┤
│  Layer 1: OS 线程 (M:N 映射)                         │
│  ┌──────┐  ┌──────┐  ┌──────┐  ┌──────┐           │
│  │  M0   │  │  M1   │  │  M2   │  │  M3   │           │
│  │ (OS)  │  │ (OS)  │  │ (OS)  │  │ (OS)  │           │
│  └──────┘  └──────┘  └──────┘  └──────┘           │
│                                                      │
│  M = min(N_CPU, max_threads)                         │
│  P = N_CPU (GOMAXPROCS)                              │
└─────────────────────────────────────────────────────┘
```

## GMP 调度模型

### G (Goroutine/协程)
```rust
struct Goroutine {
    id: u64,
    state: GoroutineState,
    stack: Stack,           // 分段栈，2KB 初始，按需增长
    saved_regs: Registers,  // 保存的寄存器状态
    actor: Option<ActorId>, // 所属 Actor（可选）
    mailboxes: Vec<MailboxRef>, // 监听的信箱
}

enum GoroutineState {
    Runnable,   // 可运行
    Running,    // 正在运行
    Waiting,    // 等待 (I/O, receive, channel)
    Dead,       // 已结束
    Syscall,    // 系统调用中
}
```

### P (Processor) — 逻辑处理器
```rust
struct Processor {
    id: usize,
    local_queue: VecDeque<Goroutine>,  // 本地运行队列
    run_queue_size: usize,              // 最多 256
    current_g: Option<Goroutine>,       // 当前运行的 G
    bound_m: Option<ThreadId>,          // 当前绑定的 M
    gc_state: GcState,                  // GC 状态
    steal_count: AtomicU64,             // 窃取计数
}
```

### M (Machine) — OS 线程
```rust
struct Machine {
    id: usize,
    thread: JoinHandle<()>,
    bound_p: Option<Processor>,    // 当前绑定的 P
    spinning: AtomicBool,           // 是否在自旋等待
    syscall_g: Option<Goroutine>,  // 系统调用中的 G
}
```

## 调度策略

### 协程调度循环
```
forever:
    1. 从 P 的本地队列取 G（LIFO，cache 友好）
    2. 每 61 次调度检查全局队列
    3. 本地为空 → 尝试从全局队列取
    4. 全局为空 → 尝试从其他 P 窃取（FIFO，偷队列尾部）
    5. 仍然为空 → 检查网络轮询器
    6. 无事可做 → M 进入自旋或休眠
```

### 抢占调度
- 在每个函数调用入口和循环回边检查抢占标志
- 每 10ms 发送抢占信号
- 抢占点：函数入口、循环回边、GC 安全点
- 挂起 G 时保存 PC 和寄存器到 G 的上下文

### 系统调用处理
1. G 进入 syscall → M 解绑 P
2. P 寻找新的 M（唤醒休眠 M 或创建新 M）
3. syscall 返回 → G 尝试重新获取 P
4. 若 P 不够 → G 放入全局队列等待

## Actor 模型

### Actor 生命周期
```rust
struct Actor {
    pid: ActorId,
    state: ActorState,
    heap: Heap,              // 私有堆
    mailbox: Mailbox,        // 消息邮箱
    monitor_links: Vec<ActorId>, // 监控链接
}

enum ActorState {
    Running,
    Waiting,
    Terminated(ExitReason),
}

struct Mailbox {
    external: MpscQueue<Message>,  // 外部写入（多生产者）
    internal: VecDeque<Message>,   // 内部消费（单消费者）
    // 批量从 external 转移到 internal 减少锁竞争
}
```

### 消息传递
- `spawn(closure)` → 创建新 Actor，返回 Pid
- `pid ! msg` → 异步发送到 Actor 邮箱
- `receive { pattern => body, ... }` → 选择性接收
- 小消息（≤64B）：复制到接收者堆
- 大消息（>64B）：引用传递，全局堆分配，引用计数

### Actor 的 GC
- 每个 Actor 拥有私有堆
- GC 只影响单个 Actor（局部暂停）
- Actor 终止 → 整个堆 O(1) 释放
- 大对象（引用传递）→ 全局 GC 管理

## async/await 编译

### 状态机转换
```rust
// 源语言
async fun fetch(url: string): string {
    val data = await http_get(url);
    return data;
}

// 编译为状态机
enum FetchStateMachine {
    State0_Start,
    State1_AwaitHttpGet { url: string },
    State2_Done { data: string },
    Poisoned,
}

impl Future for FetchStateMachine {
    type Output = string;
    fn poll(&mut self, cx: &mut Context) -> Poll<Self::Output> {
        match self {
            // 推进状态机，每个 .await 点 = 状态转换
        }
    }
}
```

## Happens-Before 内存模型

### 核心规则
1. **单协程内**：程序顺序即 happens-before 顺序
2. **消息传递**：`send(msg)` hb `receive(msg)`
3. **协程创建**：`spawn(f)` hb `f` 的第一条语句
4. **协程终止**：`f` 的最后一条语句 hb `join(f)` 返回
5. **传递性**：`A hb B` ∧ `B hb C` → `A hb C`

### Actor 间通信保证
- 发送者在发送前对消息的所有写入，接收者都可见
- 不需要 volatile 或原子操作（消息传递天然保证）
- 若引入共享内存，需额外同步原语

## 并发原语（一期留接口，二期标准库实现）

| 原语 | 说明 | 一期 |
|------|------|------|
| `spawn` | 创建 Actor/协程 | 接口预留 |
| `receive` | Actor 消息接收 | 接口预留 |
| `async/await` | 异步函数 | 接口预留 |
| `chan<T>` | 类型化通道 | 二期 |
| `select` | 多路选择 | 二期 |
| `mutex`/`rwlock` | 共享内存锁 | 二期 |

## VM 内并发接口

```rust
// 一期 VM 预留的并发 trait
trait ConcurrencyRuntime: Send + Sync {
    fn spawn_actor(&self, closure: GcRef<Closure>) -> ActorId;
    fn send(&self, pid: ActorId, msg: Value) -> Result<(), SendError>;
    fn receive(&self, patterns: &[Pattern]) -> Value;
}

// 默认实现：单线程执行器（一期）
struct SingleThreadedRuntime;
// 后续替换为：MultiThreadedRuntime
```

## 与 GC 的协调

### GC 安全点
- 每个协程只能在安全点被 GC 暂停
- 安全点：函数入口、循环回边、分配操作
- 安全点插入规则：每 1000 条指令或每个基本块末尾

### 并发 GC 握手
```
Worker 线程                    GC 线程
    │                            │
    │ ── 到达安全点 ──►          │
    │    (自旋等待)              │ 扫描 Roots
    │                            │ 标记可达对象
    │ ◄── GC 完成 ────          │
    │    (继续执行)              │
    │                            │
```

### Actor 局部 GC
- Actor 执行中 → 仅 GC 自己的堆，不影响其他 Actor
- 优势：不需要全局 STW，延迟可控
- 代价：跨 Actor 引用需特殊处理（引用计数或全局堆）