# 寄存器式 VM 指令集设计调研报告

> 目标：为类 Kotlin 语法的多范式语言（Rust 实现）设计高性能寄存器式 VM 指令集
> 性能对标：JVM / V8
> 调研日期：2026-08-05

---

## 1. Lua 5 字节码指令格式

### 1.1 概述

Lua 5.0 是第一个被广泛使用的寄存器式虚拟机。Lua 5.5 使用 **32 位定长指令**，7 位操作码，支持 6 种指令格式。

### 1.2 指令编码格式（Lua 5.5）

```
3 3 2 2 2 2 2 2 2 2 2 2 1 1 1 1 1 1 1 1 1 1 0 0 0 0 0 0 0 0 0 0
1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0
iABC:   C(8)  |  B(8)  |k| A(8)  |  Op(7)  |
ivABC:  vC(10)|  vB(6) |k| A(8)  |  Op(7)  |
iABx:         Bx(17)         | A(8)  |  Op(7)  |
iAsBx:        sBx(17)        | A(8)  |  Op(7)  |
iAx:                Ax(25)            |  Op(7)  |
isJ:                sJ(25)            |  Op(7)  |
```

### 1.3 关键设计要点

| 特性 | 值 |
|------|-----|
| 指令宽度 | 固定 32 位 |
| 操作码位数 | 7 位（最多 128 条指令） |
| 寄存器字段 | A = 8 位（最多 256 个寄存器） |
| 操作数编码 | RK 模式：bit k 区分寄存器(0)和常量(1) |
| 有符号偏移 | 使用 excess-K 偏移编码（如 sBx = 无符号值 - MAXARG_Bx/2） |
| 三地址格式 | `ADD A, B, C` 即 `R[A] = R[B] + R[C]` |

### 1.4 Lua 5.3 操作码分类

```
OP_MOVE       -- 数据移动
OP_LOADK      -- 加载常量
OP_LOADBOOL   -- 加载布尔值
OP_LOADNIL    -- 加载 nil
OP_GETUPVAL / OP_SETUPVAL  -- 上值（闭包变量）
OP_GETTABLE / OP_SETTABLE  -- 表访问
OP_ADD / OP_SUB / OP_MUL / OP_DIV / OP_MOD / OP_POW  -- 算术
OP_BAND / OP_BOR / OP_BXOR / OP_SHL / OP_SHR  -- 位运算
OP_UNM / OP_BNOT / OP_NOT / OP_LEN  -- 一元运算
OP_CONCAT     -- 字符串连接
OP_JMP        -- 跳转
OP_EQ / OP_LT / OP_LE  -- 比较
OP_TEST / OP_TESTSET   -- 测试
OP_CALL / OP_TAILCALL  -- 调用
OP_RETURN     -- 返回
OP_FORLOOP / OP_FORPREP / OP_TFORCALL / OP_TFORLOOP  -- 循环
OP_NEWTABLE   -- 创建表
OP_CLOSURE / OP_VARARG  -- 闭包和变长参数
```

### 1.5 算术宏系统

Lua 的算术指令通过宏系统实现，定义三层宏：
- 顶层：`op_arith` 分发到具体操作
- 中层：`arith_op` 处理 RK 解码和类型检查
- 底层：`arith_add` 等实际运算

---

## 2. Dalvik 字节码指令格式

### 2.1 概述

Dalvik 是 Android 的寄存器式虚拟机，使用 **16 位代码单元** 的可变宽度指令，约 237 条操作码。

### 2.2 指令格式编码

| 格式 | 语法 | 说明 |
|------|------|------|
| `10x` | `op` | 无操作数 |
| `12x` | `op vA, vB` | 2 个 4 位寄存器 |
| `11n` | `op vA, #+B` | 1 个 4 位寄存器 + 4 位立即数 |
| `11x` | `op vAA` | 1 个 8 位寄存器 |
| `10t` | `op +AA` | 8 位有符号分支偏移 |
| `20t` | `op +AAAA` | 16 位有符号分支偏移 |
| `20bc` | `op AA, kind@BBBB` | 8 位寄存器 + 16 位常量池索引 |
| `22c` | `op vA, vB, kind@CCCC` | 2 个 4 位寄存器 + 16 位常量池索引 |
| `21c` | `op vAA, kind@BBBB` | 8 位寄存器 + 16 位常量池索引 |
| `23x` | `op vAA, vBB, vCC` | 3 个 8 位寄存器 |
| `22x` | `op vAA, vBBBB` | 8 位寄存器 + 16 位寄存器 |
| `31t` | `op vAA, +BBBBBBBB` | 8 位寄存器 + 32 位分支偏移 |
| `31i` | `op vAA, #+BBBBBBBB` | 8 位寄存器 + 32 位立即数 |
| `31c` | `op vAA, kind@BBBBBBBB` | 8 位寄存器 + 32 位常量池索引 |
| `35c` | `op {vC,..,vN}, kind@BBBB` | 多寄存器 + 常量池索引（方法调用） |
| `3rc` | `op {vCCCC .. vNNNN}, kind@BBBB` | 寄存器范围 + 常量池索引 |
| `51l` | `op vAA, #+BBBBBBBBBBBBBBBB` | 8 位寄存器 + 64 位字面量 |

### 2.3 关键设计要点

| 特性 | 值 |
|------|-----|
| 代码单元 | 16 位（指令可变宽度：16/32/48/64 位） |
| 寄存器宽度 | 32 位，相邻对用于 64 位值 |
| 寄存器寻址 | 4 位（16 个）、8 位（256 个）、16 位（65536 个） |
| 参数传递 | 最后 N 个寄存器 |
| 常量池 | 分离的字符串/类型/字段/方法池 |
| 实例方法 | `this` 作为第一个参数 |

### 2.4 Dalvik 操作码部分列表

```
00 nop
01 move vA, vB
02 move/from16 vAA, vBBBB
03 move/16 vAAAA, vBBBB
04 move-wide vA, vB
...
0E return-void
0F return vAA
10 return-wide vAA
11 return-object vAA
...
12 const/4 vA, #+B
13 const/16 vAA, #+BBBB
14 const vAA, #+BBBBBBBB
15 const/high16 vAA, #+BBBB0000
...
1B const-string vAA, string@BBBB
1C const-string/jumbo vAA, string@BBBBBBBB
...
27 filled-new-array
28 filled-new-array/range
...
2B add-int/2addr vA, vB
...
44-51 aget/aput (数组访问)
...
54-59 iget/iput (实例字段)
...
60-6D sget/sput (静态字段)
...
6E-72 invoke-virtual/super/direct/static/interface
...
90-95 int-to-... 类型转换
...
B0-BB add/sub/mul/div/rem -int/float/double/long
...
```

---

## 3. V8 Ignition 字节码解释器

### 3.1 概述

Ignition 是 V8 的寄存器式字节码解释器，2017 年随 V8 5.9 发布。Ignition 字节码是所有后续编译层（Sparkplug、Maglev、TurboFan）的通用中间表示。

### 3.2 架构特点

| 特性 | 说明 |
|------|------|
| 架构类型 | 寄存器式 + 累加器 |
| 寄存器 | 虚拟寄存器 r0, r1, ... 映射到栈帧槽位 |
| 累加器 | 隐式操作数，大多数指令从累加器读取/写入累加器 |
| 寄存器分配 | 编译时固定，存储在 BytecodeArray 头部 |
| 惰性编译 | 按函数编译，首次执行时生成 |

### 3.3 核心指令模式

```
Ldar  r0     -- 从寄存器 r0 加载到累加器 (Load Accumulator from Register)
Star  r0     -- 将累加器存储到寄存器 r0 (Store Accumulator to Register)
LdaSmi [42]  -- 加载小整数到累加器
Add   r0     -- 累加器 = 累加器 + r0
Sub   r0     -- 累加器 = 累加器 - r0
CallProperty0 r0, r1, r2  -- 方法调用
Return        -- 返回累加器中的值
TestEqual r0  -- 累加器 == r0 ?
JumpIfTrue [offset]  -- 条件跳转
```

### 3.4 字节码生成

- 使用 `BytecodeGenerator` 遍历 AST，通过 `BytecodeArrayBuilder` 生成
- 字节码处理程序由 `CodeStubAssembler`（CSA）编写，TurboFan 编译为机器码
- 每个 isolate 维护全局 `interpreter dispatch table`
- 宽操作数通过 `wide` 前缀字节码支持

### 3.5 寄存器机的优势

Ignition 选择寄存器式设计的原因：
- 寄存器式产生更少的字节码指令
- 更少的指令 = 更少的调度循环迭代
- 调度（dispatch）是解释器的主要开销

---

## 4. 寄存器式 vs 栈式 VM 性能对比

### 4.1 核心研究结论

| 维度 | 栈式 VM | 寄存器式 VM | 差异 |
|------|---------|------------|------|
| 指令条数 | 基准 | 减少 35-47% | 寄存器式显著更少 |
| 字节码尺寸 | 基准 | 大 25% | 寄存器式略大 |
| 执行时间（switch dispatch） | 基准 | 快 32.3% | 寄存器式更快 |
| 执行时间（threaded dispatch） | 基准 | 快 26.5% | 寄存器式更快 |
| 额外指令加载 | 基准 | 1.07 次/条 | 可忽略 |

### 4.2 具体案例对比

**Fibonacci 计算**（每函数调用）：

| VM | 指令数 |
|----|--------|
| Lua 5.4（寄存器式） | ~8 |
| CPython 3.12（栈式） | ~17 |
| Monkey（栈式） | ~16 |

**`x += y` 操作**：

| ISA | 字节数 |
|-----|--------|
| JVM（栈式） | 4 字节 |
| WASM（栈式） | 7 字节 |
| ARM Thumb（寄存器式） | 2 字节 |
| RISC-V C（寄存器式） | 2 字节 |

### 4.3 调度开销分析

- 解释器的 dispatch loop 是性能瓶颈，占 **50-80%** 总执行时间
- 每次 `switch(op)` 是一个间接分支，CPU 分支预测困难
- 寄存器式减少了指令条数 → 减少了 dispatch 次数 → 减少了分支预测失败
- 这是寄存器式 VM 性能优于栈式 VM 的**根本原因**

### 4.4 优化调度技术

| 技术 | 说明 | 性能提升 |
|------|------|---------|
| switch dispatch | 标准 C switch 语句 | 基准 |
| computed goto | GCC/Clang 的标签值扩展 | 中等 |
| direct threaded | 指令流中存储处理程序地址 | 较大 |
| inline threaded | 将处理程序代码内联到调度循环 | 最大 |
| token threaded | 使用 token 而非地址 | 可移植 |

---

## 5. WebAssembly 指令格式设计

### 5.1 概述

WebAssembly 是栈式虚拟机，使用单字节操作码和 LEB128 可变长度编码。

### 5.2 编码格式

```
指令编码 = 单字节操作码 + 立即数（LEB128 编码）

例：
0x00  unreachable
0x01  nop
0x02  block  bt  instr* 0x0B  end
0x03  loop   bt  instr* 0x0B  end
0x04  if     bt  instr* 0x05 else instr* 0x0B  end
0x0C  br     labelidx
0x0D  br_if  labelidx
0x20  local.get  localidx
0x21  local.set  localidx
0x41  i32.const  value:i32     (LEB128 有符号)
0x42  i64.const  value:i64
0x6A  i32.add
0x6B  i32.sub
0x6C  i32.mul
```

### 5.3 关键设计要点

| 特性 | 值 |
|------|-----|
| 操作码 | 单字节（0x00-0xFF），留有大量扩展空间 |
| 立即数编码 | LEB128 无符号/有符号可变长度 |
| 结构化控制流 | block/loop/if 带显式 end 标记 |
| 类型系统 | 静态类型检查，显式类型注解 |
| 指令分类 | 控制流、数值、参数、变量、内存、表 |

### 5.4 LEB128 编码

```
无符号 LEB128：
- 每 7 位数据用一个字节编码
- 最高位为 1 表示后续还有字节
- 例：624485 → 0xE5 0x8E 0x26

有符号 LEB128：
- 最后一个字节的最高位用作符号扩展
- 支持负数紧凑编码
```

### 5.5 指令分类

```
控制指令：unreachable, nop, block, loop, if, br, br_if, br_table, return, call
数值指令：i32.add, i64.mul, f32.sqrt 等（类型前缀 + 操作）
参数指令：drop, select
变量指令：local.get, local.set, local.tee, global.get, global.set
内存指令：i32.load, i32.store, memory.size, memory.grow
```

---

## 6. 综合分析与设计建议

### 6.1 指令宽度策略对比

| VM | 策略 | 优点 | 缺点 |
|----|------|------|------|
| Lua 5 | 固定 32 位 | 解码简单快速 | 小指令浪费空间 |
| Dalvik | 16 位代码单元，可变宽度 | 紧凑高效 | 解码复杂 |
| V8 Ignition | 1-N 字节可变 | 极高密度 | 解码最复杂 |
| WASM | 单字节操作码 + LEB128 | 紧凑且可扩展 | 栈式指令多 |

### 6.2 推荐方案：固定 32 位 + 前缀扩展

基于目标语言特性（类 Kotlin 多范式，Rust 实现），建议采用：

1. **主指令宽度**：固定 32 位（类似 Lua 5）
2. **扩展指令**：通过 `wide` 前缀支持 64 位扩展指令（类似 Dalvik 的 wide 模式）
3. **操作码位数**：8 位（支持 256 条指令，留有余量）
4. **寄存器字段**：8 位（支持 256 个寄存器，对标 Lua/Dalvik）

### 6.3 指令格式定义

```
基本格式（32 位）：
┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
│ 31  │ 30  │ 29  │ 28  │ 27  │ 26  │ 25  │ 24  │
├─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┤
│                                               │
│              Format-dependent                  │
│                                               │
├─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┤
│ 7   │ 6   │ 5   │ 4   │ 3   │ 2   │ 1   │ 0   │
│                  Opcode(8)                    │
└─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘

格式 1 - RRR (三寄存器):
  [A:8][B:8][C:8][Op:8]
  
格式 2 - RRI (双寄存器 + 立即数):
  [A:8][B:8][Imm:8][Op:8]

格式 3 - RI (单寄存器 + 大立即数):
  [A:8][Imm:16][Op:8]

格式 4 - I (大立即数/偏移):
  [Imm:24][Op:8]

格式 5 - RRK (双寄存器 + 常量索引):
  [A:8][B:8][K:8][Op:8]

格式 6 - WIDE 前缀:
  WidePrefix: [A:8][B:8][C:8][Op:8]
  + 扩展字:   [Extended:24][Op:8]
```

### 6.4 推荐操作码集合

#### 6.4.1 数据移动（~10 条）
```
MOV     A, B       -- R[A] = R[B]
LOADK   A, K       -- R[A] = Constants[K]
LOADI   A, Imm     -- R[A] = Imm (小立即数)
LOADNIL A, B       -- R[A..A+B] = null
MOVW    A, B       -- 宽寄存器移动（64位）
```

#### 6.4.2 算术运算（~15 条）
```
ADD, SUB, MUL, DIV, MOD, POW
ADD_I, SUB_I, MUL_I, DIV_I     -- 立即数变体
NEG, ABS                        -- 一元运算
```

#### 6.4.3 位运算（~6 条）
```
AND, OR, XOR, SHL, SHR, NOT
```

#### 6.4.4 比较运算（~6 条）
```
EQ, NE, LT, LE, GT, GE
CMP_I                           -- 立即数比较
```

#### 6.4.5 类型转换（~8 条）
```
I2F, F2I, I2L, L2I, I2S, S2I
CHECKCAST, IS
```

#### 6.4.6 控制流（~10 条）
```
JMP      Offset      -- 无条件跳转
JMP_T    A, Offset   -- R[A] == true 时跳转
JMP_F    A, Offset   -- R[A] == false 时跳转
JMP_EQ   A, B, Off   -- R[A] == R[B] 时跳转
CALL     A, B, C     -- R[A] = R[B](R[B+1]..R[B+C-1])
TAILCALL A, B, C     -- 尾调用
RETURN   A, B        -- 返回 R[A]..R[A+B-1]
```

#### 6.4.7 对象/字段操作（~10 条）
```
GETFIELD  A, B, K    -- R[A] = R[B].field[K]
SETFIELD  A, B, K    -- R[A].field[K] = R[B]
GETSTATIC A, K       -- R[A] = Static.field[K]
SETSTATIC A, K       -- Static.field[K] = R[A]
NEW       A, K       -- R[A] = new Type[K]
NEWARRAY  A, B, K    -- R[A] = new Array[K](R[B])
```

#### 6.4.8 数组操作（~4 条）
```
AGET   A, B, C       -- R[A] = R[B][R[C]]
ASET   A, B, C       -- R[A][R[B]] = R[C]
ALEN   A, B          -- R[A] = R[B].length
```

#### 6.4.9 闭包/函数（~4 条）
```
CLOSURE  A, K        -- R[A] = closure(K)
GETUPVAL A, B        -- R[A] = UpValue[B]
SETUPVAL A, B        -- UpValue[B] = R[A]
```

#### 6.4.10 异常处理（~2 条）
```
THROW  A             -- throw R[A]
TRY    A, B, C       -- try { ... } catch(R[A]) { ... } finally { ... }
```

**总计：约 75-80 条操作码，8 位操作码空间（256）有余量。**

### 6.5 Rust 实现要点

#### 6.5.1 指令表示

```rust
/// 32 位指令
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Instruction(pub u32);

impl Instruction {
    /// 提取操作码（低 8 位）
    pub fn opcode(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
    
    /// 提取 A 字段（位 8-15）
    pub fn a(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }
    
    /// 提取 B 字段（位 16-23）
    pub fn b(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }
    
    /// 提取 C 字段（位 24-31）
    pub fn c(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }
    
    /// 提取 Bx 字段（位 8-23，16 位无符号）
    pub fn bx(self) -> u16 {
        ((self.0 >> 8) & 0xFFFF) as u16
    }
    
    /// 提取 Imm24 字段（位 8-31，24 位）
    pub fn imm24(self) -> u32 {
        (self.0 >> 8) & 0xFFFFFF
    }
    
    /// 带符号扩展的 sBx
    pub fn sbx(self) -> i32 {
        let bx = self.bx() as i32;
        bx - (0xFFFF >> 1)  // excess-K 偏移
    }
}
```

#### 6.5.2 解释器主循环

```rust
pub struct Vm {
    registers: Vec<Value>,
    constants: Vec<Value>,
    code: Vec<Instruction>,
    ip: usize,  // instruction pointer
}

impl Vm {
    pub fn run(&mut self) -> Result<Value, VmError> {
        loop {
            let inst = self.code[self.ip];
            self.ip += 1;
            
            match inst.opcode() {
                // 使用 macro_rules! 生成算术指令
                OP_ADD => {
                    let a = inst.a() as usize;
                    let b = inst.b() as usize;
                    let c = inst.c() as usize;
                    self.registers[a] = self.registers[b].add(&self.registers[c])?;
                }
                OP_JMP => {
                    let offset = inst.sbx();
                    self.ip = ((self.ip as i32) + offset - 1) as usize;
                }
                OP_RETURN => {
                    let a = inst.a() as usize;
                    return Ok(self.registers[a].clone());
                }
                // ...
            }
        }
    }
}
```

#### 6.5.3 关键优化技术

1. **computed goto（需 nightly Rust）**：
   ```rust
   #![feature(generic_label)]
   // 使用标签地址直接跳转，消除 switch 的间接分支
   ```

2. **Threaded dispatch（stable Rust）**：
   ```rust
   // 在指令流末尾存储下一条指令地址
   // 通过函数指针表实现
   static HANDLERS: [fn(&mut Vm, Instruction); 256] = [...];
   ```

3. **零成本抽象**：
   - 利用 Rust 的 `enum` 和 `match` 生成高效代码
   - 使用 `#[repr(u8)]` 确保操作码枚举紧凑

4. **内存布局优化**：
   - `Value` 使用 tagged pointer（NaN-boxing 或 指针标记）
   - 寄存器数组使用 `Vec<Value>` 预分配
   - 常量池使用 `Box<[Value]>` 避免间接访问

5. **类型特化**：
   ```rust
   // 根据类型反馈生成特化指令
   OP_ADD_INT => { /* 专门处理整数加法，跳过类型检查 */ }
   OP_ADD_FLOAT => { /* 专门处理浮点加法 */ }
   ```

### 6.6 设计决策总结

| 决策 | 推荐方案 | 理由 |
|------|---------|------|
| 指令宽度 | 固定 32 位 + 可选 wide 前缀 | 参考 Lua 5 成功经验，解码简单，Rust match 高效 |
| 操作码位数 | 8 位 | 大于 Lua 的 6/7 位，为多范式语言留足扩展空间 |
| 寄存器字段 | 8 位（256 个寄存器） | 足够大多数函数使用，对标 Dalvik 8 位寄存器 |
| 架构类型 | 纯寄存器式 | 比栈式少 47% 指令，性能优势显著 |
| 编码格式 | 多格式（RRR/RRI/RI/I） | 类似 Lua 的 iABC/iABx 设计，平衡密度和灵活性 |
| 常量编码 | RK 混合模式 | 在寄存器字段中用 1 bit 区分寄存器/常量索引 |
| 控制流 | 结构化 + 偏移跳转 | 简化验证，支持优化编译 |
| 类型系统 | 动态类型 + 可选类型特化 | 类 Kotlin 多范式，需支持泛型和类型安全 |

---

## 参考资料

- [The Implementation of Lua 5.0](https://web.tecgraf.puc-rio.br/~lhf/ftp/doc/jucs05.pdf) - Lua 5.0 寄存器式 VM 论文
- [Lua 5.5 lopcodes.h](https://lua.org/source/5.5/lopcodes.h.html) - Lua 5.5 指令编码源码
- [Dalvik Bytecode Format](https://source.android.com/docs/core/runtime/dalvik-bytecode) - Android 官方 Dalvik 文档
- [Ignition: V8 Interpreter](https://dynamic-languages-symposium.org/dls-16/program/media/McIlroy_2016_IgnitionJumpStartingAnInterpreterForV8_Dls.pdf) - V8 Ignition 设计论文
- [Virtual Machine Showdown: Stack Versus Registers](https://dl.acm.org/doi/pdf/10.1145/1328195.1328197) - 栈式 vs 寄存器式 VM 性能对比
- [WebAssembly Specification](https://webassembly.github.io/gc/core/_download/WebAssembly.pdf) - WASM 规范
- [How Bytecode VMs Actually Work](https://henry-the-frog.github.io/2026/03/21/how-bytecode-vms-actually-work/) - 字节码 VM 对比分析