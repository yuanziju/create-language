# 运行时值表示策略调研

> 调研日期: 2026-08-05
> 目标: 为寄存器式 VM（Rust 实现，支持渐进类型和 GC）选择最优的值表示方案

---

## 目录

1. [方案一：Tagged Union（带标签的联合体）](#1-方案一tagged-union带标签的联合体)
2. [方案二：Tagged Pointer（低比特标签指针）](#2-方案二tagged-pointer低比特标签指针)
3. [方案三：NaN-Boxing](#3-方案三nan-boxing)
4. [方案四：Pointer-Biased NaN-Boxing (JavaScriptCore)](#4-方案四pointer-biased-nan-boxing-javascriptcore)
5. [方案五：Ex-Boxing（Exponent Boxing）](#5-方案五ex-boxingexponent-boxing)
6. [方案六：Float Self-Tagging](#6-方案六float-self-tagging)
7. [方案七：NuN-Boxing & Pun-Boxing (SpiderMonkey)](#7-方案七nun-boxing--pun-boxing-spidermonkey)
8. [真实引擎采用情况](#8-真实引擎采用情况)
9. [学术基准测试结果](#9-学术基准测试结果)
10. [Rust 实现考量](#10-rust-实现考量)
11. [Rust NaN-Boxing Crates](#11-rust-nan-boxing-crates)
12. [GC 兼容性考量](#12-gc-兼容性考量)

---

## 1. 方案一：Tagged Union（带标签的联合体）

### 原理

显式地将 tag（判别式）和 payload（有效负载）打包在一个结构体中。这是 Rust `enum` 的自然实现方式。

### 典型内存布局

```
┌─────────────┬──────────────┬───────────┐
│  Tag (8B)   │  Payload (8B)│  Padding  │
│  discrim    │  union max   │           │
└─────────────┴──────────────┴───────────┘
总大小: 16 字节 (64-bit)
```

### QuickJS 实现

```c
// QuickJS 的 tagged union 实现
typedef union JSValueUnion {
    int32_t int32;
    double float64;
    void *ptr;
} JSValueUnion;

typedef struct JSValue {
    JSValueUnion u;
    int64_t tag;
} JSValue;
// sizeof(JSValue) == 16 字节
```

### Rust 的 Tagged Union（enum）

```rust
// 朴素实现 - 24 字节或更多
pub enum Value {
    Number(f64),      // 8B
    Text(String),     // 24B (ptr, len, cap)
    Bool(bool),       // 1B
    Nil,
    List(Vec<Value>), // 24B
}
// sizeof(Value) ≈ 32 字节 (tag + max variant)

// Box 优化后 - 约 16 字节
pub enum ValueOptimized {
    Number(f64),
    String(Box<String>),  // 8B 指针
    Bool(bool),
    Nil,
    List(Box<Vec<Value>>),
}
// sizeof(ValueOptimized) ≈ 16 字节
```

### Rust Niche 优化

Rust 编译器利用类型的"不可能位模式"来消除 tag 空间：

| 类型 | Niche | 效果 |
|------|-------|------|
| `&T`, `Box<T>` | 0x0（空指针） | `Option<&T>` 大小 = `&T` 大小 |
| `bool` | 2..255 | `Option<bool>` 仍是 1 字节 |
| `NonZeroU64` | 0 | `Option<NonZeroU64>` 大小 = 8 字节 |

Rust 编译器在 `repr(Rust)` enum 布局中有两种策略：
1. **Tagged layout**: 整数 tag 字段 + 各变体字段
2. **Niche-filling layout**: 选择一个有足够 niche 的最大变体，用 niche 值编码其他变体

### 关键引用

- Rust Nomicon - repr(Rust): https://doc.rust-lang.org/nomicon/repr-rust.html
- QuickJS 源码: JSValue 定义
- CSDN 博客: "JavaScript引擎深入剖析(一)：JSValue 的内部实现" - https://blog.csdn.net/DiDi_Tech/article/details/116956079

---

## 2. 方案二：Tagged Pointer（低比特标签指针）

### 原理

利用堆对象对齐（8 字节或 16 字节对齐）产生的低比特零位，用这些位存储类型标签。如果低比特为 0 表示指针，非 0 表示立即数（如整数）。

### 典型 64 位布局

```
指针:  ┌──────────────────────────────────────┬───┬───┬───┐
       │         48-bit 地址                  │ 0 │ 0 │ 0 │  ← 8字节对齐
       └──────────────────────────────────────┴───┴───┴───┘
                                                     ↑
                                                 3-bit tag

整数:  ┌──────────────────────────────────────┬───┬───┬───┐
       │        63-bit 有符号整数 (左移)        │ 0 │ 0 │ 1 │  ← tag=1
       └──────────────────────────────────────┴───┴───┴───┘
```

### V8 的 SMI（Small Integer）

V8 使用 tagged pointer 方案：
- **低 1 位为 0**: 指针（指向堆对象）
- **低 1 位为 1**: SMI（31 位或 32 位小整数，取决于架构）
- **double**: 堆分配（boxed），有时使用 "double field unboxing" 内联优化

```cpp
// V8 的 Smi 标记 (概念性)
// 在 64 位系统上，Smi 是 32 位有符号整数左移 32 位
// 低 32 位全是 0（即 tag 为 0x00000000）
// 指针低 1 位为 1
```

### SBCL (Steel Bank Common Lisp) 的 Lowtag 系统

SBCL 使用更复杂的低比特标签方案：

```
64-bit 系统: 4 位 lowtag
  - fixnum: 低 n-fixnum-tag-bits 位全为 0（可直接用原生指令做加减）
  - instance pointer: 特定 tag
  - function pointer: 特定 tag
  - list pointer: 特定 tag  
  - other pointer: 特定 tag
  - other-immediate: 2-bit wide lowtag（用于 widetag 编码）
```

Widetag 系统：堆对象头部有 8-bit widetag，提供更精细的类型信息。这种两层标签系统（lowtag + widetag）在 tag 空间有限的条件下提供了丰富的类型信息。

### Emacs Lisp 的 LSb Tagging

Emacs 使用低 3 位标签：
```
Tag Value:
  0 = even fixnum
  1 = odd fixnum  
  2 = symbol
  3 = cons
  4 = string
  5 = vector-like
  6 = float
```

### Racket 的标记方案

Racket 使用低 1 位标记：
- 低 1 位 = 1: 立即整数
- 低 1 位 = 0: 字对齐指针（指向堆对象，堆对象头部有 `Scheme_Type` tag）

### 优缺点

| 优点 | 缺点 |
|------|------|
| 整数运算几乎零开销（只需移位） | double 必须堆分配（boxed） |
| 类型检查只需位掩码操作 | 数值密集型程序性能差 |
| 实现简单，不依赖 IEEE 754 | 可用 tag 位有限（3-4 位） |
| 指针提取简单 | 指针有对齐要求 |

### 关键引用

- SBCL Internals - Type tags: https://www.chiark.greenend.org.uk/doc/sbcl/sbcl-internals/Type-tags.html
- Emacs Lisp Internals: https://thecloudlet.github.io/technical/project/emacs-03/
- Racket Internals: https://web.mit.edu/racket_v612/amd64_ubuntu1404/racket/doc/inside/im_values_types.html
- Python tagged pointers 讨论: https://discuss.python.org/t/using-tagged-pointers-to-support-efficient-integer-operations/87950

---

## 3. 方案三：NaN-Boxing

### 原理

IEEE 754 双精度浮点数中，NaN 有 2^51 种有效位模式，但硬件只产生一种规范 qNaN。利用这些"闲置"的 NaN 位模式来编码非浮点值（指针、整数、布尔等）。

### IEEE 754 Double 结构

```
 63  62       52  51                                                0
┌───┬───────────┬──────────────────────────────────────────────────┐
│ S │ Exponent  │                   Mantissa                       │
│ 1 │  11 bits  │                   52 bits                        │
└───┴───────────┴──────────────────────────────────────────────────┘

NaN 条件: Exponent = 0x7FF (全 1), Mantissa ≠ 0
qNaN:     bit 51 = 1
sNaN:     bit 51 = 0, Mantissa ≠ 0
```

### Eta 语言的 NaN-Boxing 布局

来源: https://github.com/lewismj/eta/blob/main/docs/guide/reference/nanboxing.md

```
Boxed 值布局:
 63  62       52  51  50  49  48  47  46                          0
┌───┬───────────┬───┬───┬─────────┬────────────────────────────────┐
│ S │ 111 1111  │ Q │ M │  Tag    │           Payload              │
│ ? │  1111     │ 1 │ 1 │ 3 bits  │          47 bits               │
└───┴───────────┴───┴───┴─────────┴────────────────────────────────┘
     ◄── BOXED_PATTERN_MASK = 0x7FFC_0000_0000_0000 ──►

is_boxed 检查: (bits & 0x7FFC_0000_0000_0000) == 0x7FFC_0000_0000_0000

Tag 值:
  Nil = 0, Char = 1, Fixnum = 2, String = 3, 
  Symbol = 4, Nan = 5, HeapObject = 6
```

### LuaJIT GC64 的 NaN-Boxing 布局

来源: LuaJIT/src/lj_obj.h v2.1 (Mike Pall)

```
高位 13 bits = 1 (0xFFF8...) 特殊 NaN 信号
bits 47..50 = itype (4-bit)
低 47 bits = payload (LJ_GCVMASK = ((uint64_t)1 << 47) - 1)

itype 标签 (按位取反):
  LJ_TNIL=~0, LJ_TFALSE=~1, LJ_TTRUE=~2, LJ_TLIGHTUD=~3,
  LJ_TSTR=~4, LJ_TUPVAL=~5, LJ_TTHREAD=~6, LJ_TPROTO=~7,
  LJ_TFUNC=~8, LJ_TTRACE=~9, LJ_TCDATA=~10, LJ_TTAB=~11,
  LJ_TUDATA=~12, LJ_TNUMX=~13
```

### ilo 语言的 Rust NaN-Boxing 实现

来源: Daniel John Morris - "NaN Boxing in Rust" (2026-03-13)
https://www.danieljohnmorris.com/writing/nan-boxing-in-rust/

```rust
const QNAN: u64       = 0x7FFC_0000_0000_0000;
const TAG_NIL: u64    = QNAN;
const TAG_TRUE: u64   = QNAN | 1;
const TAG_FALSE: u64  = QNAN | 2;
const TAG_STRING: u64 = 0x7FFD_0000_0000_0000;
const TAG_LIST: u64   = 0x7FFE_0000_0000_0000;
const TAG_RECORD: u64 = 0x7FFF_0000_0000_0000;
const TAG_OK: u64     = 0xFFFC_0000_0000_0000;
const TAG_ERR: u64    = 0xFFFD_0000_0000_0000;
const PTR_MASK: u64   = 0x0000_FFFF_FFFF_FFFF;

#[derive(Clone, Copy)]
pub(crate) struct NanVal(u64);

// 数字检查：零开销
#[inline] pub(crate) fn is_number(self) -> bool { 
    (self.0 & QNAN) != QNAN 
}

// 数字提取：零开销
#[inline] pub(crate) fn as_number(self) -> f64 { 
    f64::from_bits(self.0) 
}
```

性能数据（ilo VM）：
- NaN-boxing 前: `tot(10, 20, 30)` 约 172ns/调用
- NaN-boxing 后: 156ns/调用
- 寄存器 VM 重写后: 66ns/调用
- NaN-boxing 的关键价值：统一栈表示、Copy 语义、消除热路径的 enum 匹配

### 优缺点

| 优点 | 缺点 |
|------|------|
| double 零编码开销（直接存） | 指针提取需要掩码操作 |
| 所有值 8 字节，统一表示 | 指针 payload 被限制在 48-51 位 |
| 栈连续存储，cache 友好 | 在 Rust 中需要大量 unsafe 代码 |
| `Copy` 语义可行 | 手动引用计数管理，容易出错 |
| 无需堆分配 double | 地址空间扩展时可能出问题 |

---

## 4. 方案四：Pointer-Biased NaN-Boxing (JavaScriptCore)

### 原理

JavaScriptCore (WebKit) 使用一种改进的 NaN-boxing，通过减去一个常量来避免指针追踪时的掩码操作。

来源: https://www.libhunt.com/posts/892745-nan-boxing

- 值用 NaN-box 减去常量存储
- 指针追踪时不需要掩码（因为常量偏移抵消了 NaN 标记）
- **代价**: 浮点操作变得更昂贵（需要加回常量）

### 权衡

| 操作 | 标准 NaN-boxing | JSC NaN-boxing |
|------|----------------|----------------|
| 指针追踪 | 需要掩码 | 不需要掩码 ✓ |
| 浮点操作 | 零开销 ✓ | 需要加常量 |
| 类型检查 | 位掩码测试 | 位掩码测试 |

---

## 5. 方案五：Ex-Boxing（Exponent Boxing）

来源: Kannan Vijayan - "Exboxing: Bridging the divide between tag-boxing and NaN-boxing"
https://medium.com/@kannanvijayan/exboxing-bridging-the-divide-between-tag-boxing-and-nan-boxing-07e39840e0ca

### 原理

混合方案：利用 double 的 exponent 位而非 NaN 位来编码类型信息。解耦了 boxing 表示与虚拟地址有效位的关系。

### 关键洞察

- 不依赖地址空间的高位限制
- 大多数运行时 double 仍可作为立即数表示
- 试图融合 tagged pointer 和 NaN-boxing 的优势

---

## 6. 方案六：Float Self-Tagging

来源: Olivier Melançon, Manuel Serrano, Marc Feeley - "Float Self-Tagging" (OOPSLA 2025)
https://arxiv.org/html/2411.16544v3

### 原理

使用可逆的位变换将浮点数映射到正确包含类型标签的 tagged 值。利用实际程序中浮点数分布的非均匀性，避免绝大多数浮点数的堆分配。

### 核心结论

- 三种主流 float 编码方案（tagged pointers, NaN-boxing, NuN-boxing）各有缺陷：
  - Tagged pointers: 所有 float 必须堆分配
  - NaN/NuN-boxing: 类型检查和其他对象处理有额外运行时开销
- Self-tagging 在保持类型检查效率的同时，几乎消除了所有 float 堆分配
- 在两种 Scheme 编译器和四种微架构上评估，性能接近 NaN-boxing 且对非 float 代码影响可以忽略

### 实现要点

- 利用 float 值分布的非均匀性（大部分 float 落在特定范围）
- 对能映射到 tagged 表示的值使用可逆变换
- 对无法映射的值回退到堆分配

---

## 7. 方案七：NuN-Boxing & Pun-Boxing (SpiderMonkey)

来源: CSDN - "JavaScript引擎深入剖析(一)：JSValue 的内部实现"

SpiderMonkey 在 64-bit 系统上使用 NaN-boxing 的变体：
- **NuN-boxing**: 利用 NaN 空间存储非 double 值
- **Pun-boxing**: 另一种变体

相比 JavaScriptCore 的 pointer-biased 方法，SpiderMonkey 使用更直接的 NaN-boxing 实现。

---

## 8. 真实引擎采用情况

| 引擎 | 方案 | 64-bit 值大小 | 特点 |
|------|------|-------------|------|
| **V8 (Chrome)** | Tagged Pointer (SMI) | 8 字节 | double 堆分配；SMI 31/32-bit |
| **SpiderMonkey (Firefox)** | NaN-boxing (NuN/Pun) | 8 字节 | double 直接存储 |
| **JavaScriptCore (Safari)** | Pointer-Biased NaN-boxing | 8 字节 | 指针追踪零掩码，浮点操作有代价 |
| **LuaJIT** | NaN-boxing (GC64) | 8 字节 | 47-bit payload，4-bit itype |
| **QuickJS** | Tagged Union | 16 字节 | 简单但空间效率低 |
| **Hermes (Meta)** | NaN-boxing | 8 字节 | 类 SpiderMonkey |
| **SBCL (Common Lisp)** | Tagged Pointer (Lowtag+Widetag) | 8 字节 | 4-bit lowtag + 8-bit widetag |
| **Emacs Lisp** | Tagged Pointer (LSB) | 8 字节 | 3-bit lowtag |
| **Racket** | Tagged Pointer (1-bit) | 8 字节 | 1-bit + header tag |
| **Duktape** | NaN-boxing | 8 字节 | 嵌入式 JS 引擎 |
| **Eta** | NaN-boxing | 8 字节 | 3-bit tag, 47-bit payload |

---

## 9. 学术基准测试结果

来源: Stephen M. Watt - "Look Before You Leap: Checking in on Type Tag Checking" (arXiv:2606.05466, 2026-06-03)
https://arxiv.org/html/2606.05466v1

### 测试的表示方案

1. **Badged object headers**: 类型信息存储在堆对象头部
2. **Low-bit pointer tagging**: 低比特存类型标签
3. **NaN-boxing**: 两种不同的 NaN-boxing 布局

### 关键结论

1. **Header-based type counting 比 value-word 分类慢得多**:
   - AArch64: P1/P2 = 14.5x
   - x86-64: P1/P2 = 36.8x

2. **Header-based integer summation 比 immediate integer summation 慢得多**:
   - 低比特标签: 6.3x (AArch64) / 7.5x (x86-64)
   - NaN-boxing: 8.9x (AArch64) / 4.5x (x86-64)

3. **总体结论**:
   - Low-bit tagging: 对于以符号计算为主的工作负载，是最简单且通常最快的选择
   - NaN-boxing: 访问成本接近 low-bit tagging，且避免了普通浮点值的堆分配
   - 几个本地位操作通常比打开堆对象获取 tag 或小值更便宜

---

## 10. Rust 实现考量

### Rust enum 的默认布局

```rust
// 默认 repr(Rust) 枚举
enum Foo {
    A(u32),
    B(u64),
    C(u8),
}
// 典型布局: 
// struct FooRepr { data: u64, tag: u8 }
// sizeof = 16 (对齐到 8)
```

### repr(C) 枚举

```rust
#[repr(C)]
enum MyEnum {
    A(u32),
    B(f32, u64),
    C { x: u32, y: u64 },
}
// 布局 = C struct { tag: repr(Int) enum, union { payloads } }
```

### NaN-Boxing 在 Rust 中的挑战

来源: Daniel John Morris - ilo 项目经验

1. **大量 unsafe 代码**: 指针转换、位操作、手动引用计数
2. **Borrow checker 无法帮助**: 原始指针操作绕过了 Rust 的安全保证
3. **内存管理复杂**: 必须手动管理 Rc 引用计数
4. **寄存器访问需要 unsafe**: `get_unchecked` 避免边界检查
5. **性能收益显著**: 但需要大量测试来保证正确性

```rust
// 典型 unsafe 代码示例
unsafe fn as_heap_ref<'a>(self) -> &'a HeapObj {
    let ptr = (self.0 & PTR_MASK) as *const HeapObj;
    unsafe { &*ptr }
}

#[inline(always)]
fn clone_rc(self) {
    if self.is_heap() {
        let ptr = (self.0 & PTR_MASK) as *const HeapObj;
        unsafe { Rc::increment_strong_count(ptr); }
    }
}

#[inline(always)]
fn drop_rc(self) {
    if self.is_heap() {
        let ptr = (self.0 & PTR_MASK) as *const HeapObj;
        unsafe { Rc::decrement_strong_count(ptr); }
    }
}
```

### 推荐：Tagged Union + Niche 优化的混合方案

利用 Rust 编译器的 Niche 优化，可以用安全的 `enum` 实现接近 NaN-boxing 的效果：

```rust
// 利用 NonZeroU64 的 niche 来消除 tag 开销
use std::num::NonZeroU64;

// 堆指针永远不会是 0，所以 NonZeroU64 有 niche
// Option<NonZeroU64> 大小 = 8 字节

// 设计思路：
// - 用 NonZeroU64 存堆指针（tag 隐含在 niche 中）
// - 用特定 niche 值编码 nil, true, false 等
// - double 用单独的变体（需要额外 tag 空间）
```

---

## 11. Rust NaN-Boxing Crates

| Crate | 描述 | 状态 |
|-------|------|------|
| **nanbox** (Marwes) | 宏生成安全的 NaN-boxed 类型 | Alpha 质量，2017 年最后更新 |
| https://github.com/Marwes/nanbox | | 22 stars |
| **tagged-box** (Kixiron) | `no_std`, 零依赖 NaN-boxing + tagged pointer | 2020 年，31 commits |
| https://github.com/Kixiron/tagged-box | 宏接口安全创建 NaN-boxed enum | |
| **nan_boxed** (crates.io) | 超简单的 NaN-boxing crate | v1.2.0, 无依赖 |
| https://crates.io/crates/nan_boxed | | |
| **nanobox** (LemonHX) | NanoBox 优化：小项栈上，大项堆上 | v0.1.0, 2024 年 |

### tagged-box 使用示例

```rust
use tagged_box::{tagged_box, TaggableContainer, TaggableInner};

tagged_box! {
    #[derive(Debug, Clone, PartialEq)]
    pub struct Container, pub enum Item {
        String(String),
        Numbers(i32, f32),
        Nothing,
        Struct {
            float: f32,
            boolean: bool,
        },
    }
}

let container = Container::from(String::from("Hello from tagged-box!"));
```

---

## 12. GC 兼容性考量

### 各方案对 GC 的影响

| 方案 | GC 根扫描 | 写屏障 | 指针追踪 |
|------|----------|--------|---------|
| Tagged Union | 需要区分 tag 和指针 | 需要 | 直接 |
| Tagged Pointer | 低比特掩码后即指针 | 需要 | 掩码后直接 |
| NaN-Boxing | 需要区分 NaN-boxed 值和 double | 需要 | 掩码后需要规范化（符号扩展） |

### 关键 GC 问题

1. **指针识别**: GC 必须能精确识别哪些值是堆指针
   - Tagged Pointer: 检查低比特
   - NaN-Boxing: 检查 `is_boxed()` 条件
   - Tagged Union: 检查 tag 字段

2. **指针规范化**: 在 x86-64 上，指针必须是 canonical form
   - NaN-Boxing 中指针存储在 NaN payload 中，提取后需要符号扩展
   - 方案: `(ptr << 16) >> 16` (算术右移)

3. **LAM/UAI (Linear Address Masking)**: 新硬件特性
   - 允许在指针高位存储元数据
   - 但需要 `mmap` 配合，且比较指针时需要小心

4. **写屏障优化**: 
   - 如果值是不可变的立即数（如 fixnum），可以跳过写屏障
   - Tagged Pointer 方案的 fixnum 检查只需位掩码

---

## 附录：关键资源链接

- Rust Nomicon (repr/Rust): https://doc.rust-lang.org/nomicon/repr-rust.html
- Rust Reference (Type Layout): https://rustwiki.org/en/reference/type-layout.html
- Rust RFC 2195 (Really Tagged Unions): https://github.com/rust-lang/rfcs/blob/master/text/2195-really-tagged-unions.md
- SBCL Internals (Type Tags): https://www.chiark.greenend.org.uk/doc/sbcl/sbcl-internals/Type-tags.html
- Eta NaN-Boxing Memory Layout: https://github.com/lewismj/eta/blob/main/docs/guide/reference/nanboxing.md
- Daniel Morris - NaN Boxing in Rust: https://www.danieljohnmorris.com/writing/nan-boxing-in-rust/
- Float Self-Tagging (arXiv): https://arxiv.org/html/2411.16544v3
- Type Tag Checking Benchmark (arXiv): https://arxiv.org/html/2606.05466v1
- Kantan Vijayan - Exboxing: https://medium.com/@kannanvijayan/exboxing-bridging-the-divide-between-tag-boxing-and-nan-boxing-07e39840e0ca
- JS Value 内部实现 (CSDN): https://blog.csdn.net/DiDi_Tech/article/details/116956079
- JS Tagged Pointer & NaN Boxing (韩文): https://witch.work/ko/posts/javascript-trip-of-js-value-tagged-pointer-nan-boxing
- Emacs Lisp Internals: https://thecloudlet.github.io/technical/project/emacs-03/
- Racket Values & Types: https://web.mit.edu/racket_v612/amd64_ubuntu1404/racket/doc/inside/im_values_types.html
- Python Tagged Pointers 讨论: https://discuss.python.org/t/using-tagged-pointers-to-support-efficient-integer-operations/87950
- Rust Niche 优化 (掘金): https://juejin.cn/post/7629263887314501675
- Rust Compiler Team: Enum Layout Optimization: https://github.com/rust-lang/compiler-team/issues/922
- NaN-Boxing 讨论 (HN): https://news.ycombinator.com/item?id=46666262
- Pointer Tagging 讨论 (HN): https://news.ycombinator.com/item?id=45650874