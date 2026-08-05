# 内存模型 (Memory Model)

## 堆布局

```
┌─────────────────────────────────────────────────────────────┐
│                        GC Heap                               │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Young Generation (Nursery)                │   │
│  │  ┌──────────────────────┬────────────────────────┐   │   │
│  │  │        Eden          │  Survivor (From/To)    │   │   │
│  │  │     (新对象分配)       │  (存活对象晋升缓冲)      │   │   │
│  │  │     ~2-3 MB          │  ~1 MB each             │   │   │
│  │  └──────────────────────┴────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Old Generation                            │   │
│  │                                                       │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐                 │   │
│  │  │  Region  │ │  Region  │ │  Region  │   ...          │   │
│  │  │  (空闲)  │ │  (已用)  │ │  (部分)  │                 │   │
│  │  └─────────┘ └─────────┘ └─────────┘                 │   │
│  │                                                       │   │
│  │  ┌─────────────────────────────────────────────────┐ │   │
│  │  │             Card Table (卡表)                     │ │   │
│  │  │  每 512 字节堆 = 1 字节卡片                      │ │   │
│  │  │  dirty = 老生代对象可能持有新生代引用             │ │   │
│  │  └─────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │            Large Object Space (大对象区)               │   │
│  │  > 64KB 的对象直接分配于此，晋升时直接归入老生代       │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 对象头

每个堆对象 8 字节头部：

```
┌──────────────────────────────────────────────┐
│  Object Header (8 bytes)                      │
│  ┌──────┬──────┬──────┬──────────────────┐   │
│  │ mark │ age  │ size │ class_index      │   │
│  │ 1b   │ 4b   │ 11b  │ 48b → 256TB     │   │
│  └──────┴──────┴──────┴──────────────────┘   │
│                                               │
│  mark: 标记位 (GC 标记阶段)                    │
│  age:  年龄 (0-15, 达到阈值晋升)              │
│  size: 对象大小 (以 8 字节为单位, 最大 16KB)  │
│  class_index: 类型表索引 (指向 Class 元数据)   │
└──────────────────────────────────────────────┘
```

超大对象（>16KB）使用扩展头：额外 8 字节存储完整 size。

## 分配流程

```
allocate(type, size)
    │
    ├─ size > 64KB? ──► Large Object Space (bump pointer)
    │
    ├─ Eden 有空间? ──► bump pointer 分配 (极快，几条指令)
    │
    └─ Eden 满 ──► 触发 Minor GC
                       │
                       ├─ 根扫描 (stack roots + global roots)
                       ├─ 复制存活对象到 Survivor
                       ├─ 晋升年龄 >= 阈值的对象 → Old Gen
                       ├─ 交换 Survivor From/To
                       └─ Eden 清空，重新分配
```

## GC 循环

### Minor GC (新生代)
1. 扫描 GC Roots（VM 栈、全局变量表）
2. 从 Eden + Survivor(From) 复制存活对象到 Survivor(To)
3. 年龄递增，检查晋升阈值
4. 交换 From/To，清空 Eden
5. 耗时：~1-5ms（取决于存活对象数量）

### Major GC (老生代)
1. **初始标记** (STW, ~1ms)：标记 GC Roots 直接可达对象
2. **并发标记** (并发)：遍历对象图，标记所有可达对象
3. **最终标记** (STW, ~1ms)：处理并发标记期间的变更
4. **清除** (并发)：回收未标记对象
5. **碎片检测**：计算碎片率 = 1 - (最大连续空闲 / 总空闲)
   - 碎片率 < 35%：结束
   - 碎片率 >= 35%：触发 Lisp2 滑动压缩

### 全堆 GC (Full GC)
1. STW
2. Minor GC → Major GC (含压缩)
3. 大对象空间回收
4. 耗时：~10-100ms（取决于堆大小）

## 写屏障

```rust
// Card table: 每 512 字节堆一个 card
// 当老生代对象写入引用字段时调用
fn write_barrier(object: *mut Object, field_offset: usize, new_value: Value) {
    // 1. 写入值
    unsafe { object.add(field_offset).write(new_value); }

    // 2. 如果老生代对象写入了新生代引用，标记卡片
    if object.is_old() && new_value.is_young() {
        let card_index = object as usize >> 9; // / 512
        card_table[card_index] = DIRTY;
    }
}
```

## 碎片度量

```
碎片率 = 1 - (最大连续空闲块大小 / 空闲总大小)
触发压缩：碎片率 > 0.35 或 最大连续空闲 < 总空闲的 10%

碎片检测时机：每次 Major GC 清除阶段结束后
压缩算法：Lisp2 滑动压缩
  - 第一遍：计算每个对象的新地址
  - 第二遍：更新所有引用指针
  - 第三遍：移动对象到新位置
```

## 可插拔 GC 接口

```rust
trait GarbageCollector: Send + Sync {
    fn allocate(&self, size: usize, class_index: u32) -> *mut u8;
    fn collect(&self, roots: &dyn RootProvider);
    fn stats(&self) -> GcStats;
    fn name(&self) -> &str;
}

trait RootProvider {
    fn for_each_root(&self, f: &mut dyn FnMut(*mut Value));
}

trait Heap: Send + Sync {
    fn alloc(&self, size: usize) -> *mut u8;
    fn capacity(&self) -> usize;
    fn used(&self) -> usize;
}

// 默认实现：分代标记-压缩
struct GenerationalGc {
    young: RwLock<YoungGen>,
    old: RwLock<OldGen>,
    card_table: RwLock<CardTable>,
    config: GcConfig,
}
```

## 渐进类型的内存影响

- 编译期确定类型 → 栈上直接用原生类型，不经过 `Value`
- 运行时动态类型 → 走 `Value` 枚举
- 类型特化：编译器给已知类型的函数生成多个版本（单态化）