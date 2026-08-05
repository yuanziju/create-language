# GC 实现策略调研报告

> 调研日期：2026-08-05
> 目标：为 Rust 实现的寄存器式 VM 设计可插拔、分代标记-压缩、支持并发的 GC 系统

---

## 目录

1. [可插拔 GC 接口设计](#1-可插拔-gc-接口设计)
2. [分代垃圾回收策略](#2-分代垃圾回收策略)
3. [标记-压缩算法详解](#3-标记-压缩算法详解)
4. [碎片检测与压缩触发阈值](#4-碎片检测与压缩触发阈值)
5. [写屏障与记忆集技术](#5-写屏障与记忆集技术)
6. [并发标记算法](#6-并发标记算法)
7. [Rust GC 实现参考](#7-rust-gc-实现参考)
8. [推荐架构设计](#8-推荐架构设计)

---

## 1. 可插拔 GC 接口设计

### 1.1 JVM 的 GC 接口（JEP 304）

Java 10 通过 JEP 304 引入了正式的 GC 接口，其核心设计思路：

- **核心抽象类 `CollectedHeap`**：每个 GC 实现必须继承此类，驱动 GC 与 HotSpot 其余部分的交互
- **必须提供的组件**：
  - `CollectedHeap` 子类 — 堆管理
  - `BarrierSet` 子类 — 实现各类运行时屏障（读/写屏障）
  - `CollectorPolicy` — 回收策略
  - `GCInterpreterSupport` — 解释器屏障
  - `GCC1Support` / `GCC2Support` — JIT 编译器屏障
  - `MemoryService` 及相关的内存池、内存管理器
- **共享代码**：通过 helper 类实现跨 GC 的代码复用（如 Card Table 支持）
- **目标**：新增 GC 只需实现一组文档化的接口，无需修改 HotSpot 内部代码

### 1.2 为寄存器式 VM 设计的可插拔 GC 接口

借鉴 JVM 的经验，为 Rust VM 设计如下 trait 层次：

```rust
/// GC 核心接口 — 所有 GC 实现必须提供
pub trait GarbageCollector: Send + Sync {
    /// 分配内存
    fn allocate(&self, size: usize, ty: ObjectType) -> GcResult<*mut u8>;
    /// 触发 GC
    fn collect(&self, reason: GcReason) -> GcResult<()>;
    /// 获取 GC 统计信息
    fn stats(&self) -> GcStats;
    /// GC 类型标识
    fn name(&self) -> &str;
}

/// 堆抽象 — 管理内存布局
pub trait Heap: Send + Sync {
    /// 分配新对象
    fn allocate(&mut self, size: usize) -> Option<*mut u8>;
    /// 获取堆大小
    fn size(&self) -> usize;
    /// 获取已用内存
    fn used(&self) -> usize;
    /// 扩展堆
    fn expand(&mut self, additional: usize) -> bool;
    /// 收缩堆
    fn shrink(&mut self, amount: usize) -> bool;
}

/// 写屏障接口 — 分代 GC 必需
pub trait WriteBarrier: Send + Sync {
    /// 记录引用写入（post-write barrier）
    fn record_write(&self, src: *mut u8, dst: *mut u8);
    /// 记录引用读取（pre-read barrier，SATB 所需）
    fn record_read(&self, obj: *mut u8);
}

/// 根枚举接口 — 提供 GC Roots
pub trait RootProvider {
    /// 枚举所有 GC Roots
    fn enumerate_roots(&self, f: &mut dyn FnMut(Root));
}

/// 对象遍历接口 — 遍历对象引用字段
pub trait ObjectTracer {
    /// 遍历给定对象的所有引用字段
    fn trace_object(&self, obj: *mut u8, f: &mut dyn FnMut(*mut u8));
}
```

### 1.3 插件注册机制

```rust
/// GC 工厂 — 用于创建 GC 实例
pub trait GcFactory: Send + Sync {
    fn create(&self, config: &GcConfig) -> Box<dyn GarbageCollector>;
    fn name(&self) -> &str;
}

/// 全局 GC 注册表
pub struct GcRegistry {
    factories: HashMap<String, Box<dyn GcFactory>>,
}

impl GcRegistry {
    pub fn register(&mut self, factory: Box<dyn GcFactory>) { ... }
    pub fn create(&self, name: &str, config: &GcConfig) -> Option<Box<dyn GarbageCollector>> { ... }
    pub fn available(&self) -> Vec<&str> { ... }
}
```

---

## 2. 分代垃圾回收策略

### 2.1 弱分代假设（Weak Generational Hypothesis）

垃圾回收的分代理论基础：
- **大多数对象朝生夕死**：90%+ 的对象在创建后很快变为垃圾
- **少数对象长期存活**：存活越久的对象，继续存活的概率越大
- **新生代回收频率高、成本低**：只需扫描少量存活对象
- **老生代回收频率低、成本高**：但每次回收可释放大量内存

### 2.2 分代布局设计

```
┌─────────────────────────────────────────────────────────┐
│                    Managed Heap                          │
├──────────────┬──────────────┬────────────────────────────┤
│  Nursery     │  Survivor    │  Old Generation            │
│  (Eden)      │  (From/To)   │  (Tenured)                │
│  1-4 MB      │  0.5-2 MB    │  Growable                  │
├──────────────┴──────────────┴────────────────────────────┤
│  Young Generation               Old Generation           │
└─────────────────────────────────────────────────────────┘
```

### 2.3 分代回收流程

**Minor GC（新生代回收）**：
1. 从 GC Roots + Remembered Set（老→新引用）开始标记
2. 将 Eden + From Survivor 中的存活对象复制到 To Survivor
3. 超过晋升年龄（默认 2-3 次 survival）的对象晋升到老生代
4. 清空 Eden 和 From Survivor，交换 From/To 角色

**Major GC（老生代回收）**：
1. 并发标记：从 GC Roots 开始标记所有可达对象
2. 最终标记（STW）：处理并发期间的修改
3. 压缩（按需）：当碎片率超过阈值时触发
4. 清扫：回收未标记对象的内存

### 2.4 晋升策略

- **晋升年龄（Promotion Age）**：对象在新生代存活次数阈值，建议默认 2-3
- **大对象直接晋升**：超过阈值（如 64KB）的对象直接分配到老生代
- **动态年龄调整**：根据 Survivor 空间占用率动态调整晋升阈值

---

## 3. 标记-压缩算法详解

### 3.1 算法对比

| 算法 | 遍历次数 | 额外空间 | 是否保持顺序 | 复杂度 |
|------|---------|---------|-------------|--------|
| 双指针 (Two-Finger) | 2 | 0 | 任意顺序 | O(n) |
| Lisp2 | 3 | 每个对象 1 个 forwarding 域 | 滑动（保持顺序） | O(n) |
| 引线法 (Threading) | 2 | 0 | 滑动 | O(n) |
| 单次遍历 | 1 | 大 | 任意 | O(n) |

### 3.2 Lisp2 算法（推荐）

Lisp2 是应用最广泛的标记-压缩算法，需要三次遍历，但在压缩算法中吞吐量最高。

**第一遍：计算转发地址（computeLocations）**
```
scan = heap_start
free = heap_start
while scan < heap_end:
    if is_marked(scan):
        forwarding_address(scan) = free
        free += size(scan)
    scan += size(scan)
```

**第二遍：更新引用（updateReferences）**
```
# 更新根引用
for each root in roots:
    if *root != null:
        *root = forwarding_address(*root)

# 更新堆内引用
scan = heap_start
while scan < heap_end:
    if is_marked(scan):
        for each field in pointer_fields(scan):
            if *field != null:
                *field = forwarding_address(*field)
    scan += size(scan)
```

**第三遍：移动对象（relocate）**
```
scan = heap_start
while scan < heap_end:
    if is_marked(scan):
        dest = forwarding_address(scan)
        memmove(dest, scan, size(scan))
        unset_marked(dest)
    scan += size(scan)
```

### 3.3 优化：自适应扫描

来自 Jikes RVM 的研究：当存活对象密集时直接扫描堆，稀疏时扫描位图（bitmap），可显著减少扫描时间。

### 3.4 对象头部设计

```
┌──────────────────────────────────────┐
│  Object Header (8-16 bytes)          │
├──────────────────────────────────────┤
│  Mark Bit       (1 bit)              │
│  Age            (4 bits, 0-15)       │
│  Forwarding Ptr (pointer-sized)      │
│  Size           (remaining bits)     │
│  Type ID        (optional)           │
├──────────────────────────────────────┤
│  Object Data                         │
└──────────────────────────────────────┘
```

---

## 4. 碎片检测与压缩触发阈值

### 4.1 碎片度量指标

**碎片率（Fragmentation Ratio）**：
```
fragmentation = (total_free - max_contiguous_free) / total_free
```
- 值域 [0, 1)，0 表示无碎片，接近 1 表示严重碎片

**平均空闲块大小（Average Free Block Size）**：
```
avg_free_block = total_free / num_free_blocks
```
- 该值过小说明碎片化严重（Android ART 的做法）

**最大连续空闲块比例**：
```
contiguous_ratio = max_contiguous_free / total_free
```
- 低于阈值说明无法满足大对象分配

### 4.2 触发阈值建议

| 参数 | 建议值 | 说明 |
|------|-------|------|
| 碎片率阈值 | 0.30 - 0.50 | 超过此值触发压缩 |
| 最小空闲块比例 | < 5% 堆大小 | 平均空闲块过小触发 |
| 最大连续空闲比例 | < 10% 总空闲 | 无法分配大对象触发 |
| 分配失败即时触发 | — | 分配失败时立即检测并触发 |

### 4.3 检测机制

```rust
struct FragmentationMonitor {
    /// 碎片率阈值（0.0 - 1.0）
    threshold: f64,              // 默认 0.35
    /// 最小连续空闲比例
    min_contiguous_ratio: f64,   // 默认 0.10
    /// 上次检测时间
    last_check: Instant,
    /// 检测间隔
    check_interval: Duration,    // 默认 5 秒
}

impl FragmentationMonitor {
    fn should_compact(&self, heap: &dyn Heap) -> bool {
        let stats = heap.fragmentation_stats();
        let frag_ratio = (stats.total_free - stats.max_contiguous_free)
                         as f64 / stats.total_free as f64;
        let contiguous_ratio = stats.max_contiguous_free as f64
                               / stats.total_free as f64;

        frag_ratio > self.threshold
        || contiguous_ratio < self.min_contiguous_ratio
    }
}
```

### 4.4 增量/部分压缩

参考 G1 GC 的混合回收策略：不一次性压缩整个老生代，而是每次选择垃圾最多的若干个区域进行压缩，分摊停顿时间。

---

## 5. 写屏障与记忆集技术

### 5.1 为什么需要写屏障

分代 GC 面临的核心问题：**老生代对象可能引用新生代对象**（跨代引用）。如果每次 Minor GC 都扫描整个老生代，STW 时间会急剧增加。记忆集（Remembered Set）记录了老生代→新生代的引用，使 Minor GC 只需扫描记忆集。

### 5.2 卡表（Card Table）实现

**最主流的记忆集实现**，被 CMS、G1、Parallel Scavenge 等广泛采用：

```
堆布局：
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ 0 │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │ 7 │  ← Card Pages (512 bytes each)
└───┴───┴───┴───┴───┴───┴───┴───┘

卡表：
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ 0 │ 1 │ 0 │ 0 │ 1 │ 0 │ 0 │ 0 │  ← 1 = dirty (contains cross-gen refs)
└───┴───┴───┴───┴───┴───┴───┴───┘
```

**关键参数**：
- 卡页大小：512 字节（业界标准，被 HotSpot 采用）
- 空间开销：堆大小的 1/512
- 标记操作：`card_table[addr >> 9] = DIRTY`（仅需 2-3 条指令）

### 5.3 写屏障实现

**Post-write barrier（写入后屏障）**：
```rust
fn write_barrier(src: *mut u8, dst: *mut u8) {
    // 1. 执行实际写入
    *src = dst;
    // 2. 如果 src 在老生代，dst 在新生代，标记卡表
    if is_old_generation(src) && is_young_generation(dst) {
        let card_index = (src as usize - heap_start) >> CARD_SHIFT;
        card_table[card_index] = DIRTY;
    }
}
```

**优化：两指令写屏障（Urs Hölzle, 1993）**：
通过放宽不变式（允许标记相邻卡页），省去计算精确地址的指令，将写屏障从 3 条指令减少到 2 条。

### 5.4 记忆集维护

- **更新时机**：每次引用字段赋值时，由写屏障触发
- **Minor GC 时使用**：扫描所有脏卡页中的对象，找到跨代引用
- **清理**：Minor GC 完成后清空卡表

---

## 6. 并发标记算法

### 6.1 SATB（Snapshot-At-The-Beginning）算法

G1 GC 采用 SATB 算法，核心思想：在并发标记开始时拍一个逻辑快照，标记所有在快照时刻存活的对象。

**三色标记抽象**：
- **白色**：尚未访问（可能是垃圾）
- **灰色**：已访问但子节点未扫描完
- **黑色**：已访问且子节点已扫描完

**SATB 关键规则**：
- 并发标记期间，对黑色对象写入白色引用时，通过 pre-write barrier 保存旧值
- 旧值作为额外的 GC Root 被重新标记
- 保证不漏标（不会错误回收存活对象），但可能产生浮动垃圾

**执行阶段**：
1. **初始标记（STW）**：标记 GC Roots 直接可达的对象
2. **并发标记**：从灰色对象开始并发遍历对象图
3. **最终标记（STW）**：处理 SATB 队列中的剩余引用
4. **清理**：回收未标记对象

### 6.2 增量并发标记（CMS 风格）

CMS 采用增量更新（Incremental Update）算法：
- 并发标记期间，对黑色对象写入白色引用时，将黑色对象重新标记为灰色
- 优点：浮动垃圾少
- 缺点：需要更多重新扫描

### 6.3 并发标记的三色不变式

| 算法 | 不变式 | 写屏障类型 | 浮动垃圾 | 典型实现 |
|------|--------|-----------|---------|---------|
| SATB | 快照不变式 | Pre-write barrier | 较多 | G1, Shenandoah |
| Incremental Update | 强三色不变式 | Post-write barrier | 较少 | CMS |

### 6.4 为 VM 推荐：SATB + 并发标记

对于寄存器式 VM 推荐 SATB 方案：
- 实现相对简单
- pre-write barrier 开销可控
- 适合作为老生代并发标记的基础
- 后续可演进为更精细的增量更新方案

---

## 7. Rust GC 实现参考

### 7.1 gc-arena（kyren）

**核心特性**：
- 封闭竞技场（Arena）内安全增量 GC
- 基于"生成性"（Generativity）生命周期标记：`Gc<'gc, T>` 指针无法逃逸 Arena
- 增量标记-清除算法，类似 Lua 5.4
- 通过 `Collect` trait 自动追踪对象图
- 零成本指针：`Gc<'gc, T>` 是裸指针大小，实现 `Copy`
- 不支持多线程分配和收集
- 不支持移动对象（无压缩）

**使用场景**：
- Ruffle（Adobe Flash Player 模拟器）的 ActionScript VM
- Piccolo（无栈 Lua 运行时）
- 适合作为 VM 内 GC 的参考实现

**核心 API**：
```rust
let arena = Arena::<Rootable![Gc<'gc, MyStruct>]>::new(|mc| {
    Gc::new(mc, MyStruct { ... })
});

arena.mutate(|mc, root| {
    // 在封闭作用域内操作 GC 对象
    let new_obj = Gc::new(mc, ...);
});
```

### 7.2 runmat-gc

**核心特性**：
- 分代垃圾回收器
- 支持可选指针压缩
- 显式写屏障：VM 在写入点调用 `gc_record_write(old, new)`
- `WriteBarrierManager` 存储卡/槽元数据
- Minor GC 仅扫描记忆集位置

**参考价值**：展示了在 Rust 中实现一个完整的分代 GC 系统，包括写屏障管理和记忆集维护。

### 7.3 Rust 实现 GC 的要点

**安全性**：
- 使用 `unsafe` 块管理原始指针（GC 本质上需要绕过 Rust 所有权）
- 通过类型系统（如 gc-arena 的生成性生命周期）确保 GC 指针不会逃逸
- `Collect` trait 用于安全追踪对象图

**性能**：
- 使用 `*mut u8` 裸指针实现零成本 GC 引用
- 避免在 mutator 路径上引入额外开销
- 写屏障应尽可能精简（2-4 条指令）

**并发**：
- `Arc<Mutex<Heap>>` 或 `RwLock<Heap>` 保护共享堆状态
- 使用 `AtomicBool` 实现全局 GC 暂停标志
- 使用 `thread::park/unpark` 或 `std::sync::Barrier` 协调 STW

**内存布局**：
```rust
struct Heap {
    young_start: *mut u8,    // 新生代起始
    young_end: *mut u8,      // 新生代结束
    old_start: *mut u8,      // 老生代起始
    old_end: *mut u8,        // 老生代结束
    card_table: Vec<AtomicU8>, // 卡表
    mark_bitmap: Bitmap,     // 标记位图
    free_list: FreeList,     // 空闲链表
}
```

---

## 8. 推荐架构设计

### 8.1 算法组合

| 阶段 | 算法 | 理由 |
|------|------|------|
| 新生代 Minor GC | 复制算法（Cheney） | 大部分对象死亡，复制成本低，天然无碎片 |
| 老生代标记 | SATB 并发标记 | 低停顿，实现复杂度适中 |
| 老生代清理 | 标记-清除 | 存活率高时清除效率高 |
| 老生代压缩 | Lisp2 滑动压缩 | 保持顺序，碎片阈值触发，STW |
| 跨代引用 | 卡表 + Post-write barrier | 实现简单，空间开销低（1/512） |

### 8.2 碎片阈值建议

| 参数 | 建议值 |
|------|-------|
| 碎片率阈值 | 35%（超过触发压缩） |
| 最大连续空闲比例 | < 10% 总空闲时触发 |
| 分配失败 | 立即触发 Full GC + 压缩 |
| 检测周期 | 每次 Major GC 后检测 |

### 8.3 核心接口摘要

```rust
// 最小可插拔 GC 接口
pub trait GarbageCollector: Send + Sync {
    fn allocate(&self, size: usize, ty: ObjectType) -> GcResult<*mut u8>;
    fn collect(&self, reason: GcReason) -> GcResult<()>;
    fn stats(&self) -> GcStats;
    fn name(&self) -> &str;
}

// 写屏障（分代 GC 必需）
pub trait WriteBarrier: Send + Sync {
    fn record_write(&self, src: *mut u8, dst: *mut u8);
    fn record_read(&self, obj: *mut u8);  // SATB pre-barrier
}

// 根提供者
pub trait RootProvider {
    fn enumerate_roots(&self, f: &mut dyn FnMut(Root));
}

// 对象追踪器
pub trait ObjectTracer {
    fn trace_object(&self, obj: *mut u8, f: &mut dyn FnMut(*mut u8));
}
```

### 8.4 Rust 实现路线图

1. **Phase 1**：实现基础堆 + 标记-清除 GC（不压缩）
2. **Phase 2**：添加 Lisp2 压缩算法 + 碎片检测
3. **Phase 3**：引入分代（新生代复制 + 老生代标记-清除-压缩）
4. **Phase 4**：实现卡表写屏障 + 记忆集
5. **Phase 5**：实现 SATB 并发标记
6. **Phase 6**：可插拔接口 + 多 GC 实现共存

### 8.5 参考资源

- JEP 304: Garbage Collector Interface — https://openjdk.org/jeps/304
- Lisp2 参考实现 — https://github.com/munificent/lisp2-gc
- gc-arena — https://github.com/kyren/gc-arena
- runmat-gc — https://github.com/runmat-org/runmat
- G1 GC Tuning Guide — https://docs.oracle.com/javase/8/docs/technotes/guides/vm/gctuning/
- 写屏障论文 — A Fast Write Barrier for Generational Garbage Collectors (Hölzle, 1993)
- 分代并发 GC 论文 — A Generational Mostly-Concurrent Garbage Collector (Printezis & Detlefs, 2000)