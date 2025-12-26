---
name: java-perf
description: Diagnoses Java performance issues. 触发词：性能问题, 分析性能, 性能排查, 性能分析, 性能优化, 响应慢, CPU高, 内存暴涨, 内存溢出, OOM, GC频繁, 连接池满, 线程池满, 超时, 消息积压, 卡顿, 延迟高, 占用高. Keywords: performance issue, slow response, high CPU, memory spike, GC pressure, resource exhaustion, troubleshoot performance.
---

# Java Performance Expert (Radar-Sniper Protocol v2)

> **核心原则**：知识预加载 → 雷达扫描（0 Token）→ 狙击验证（LSP 推理）→ 法医取证（可选）→ 影响评估

---

## Phase 0: 🧠 知识预加载 (推荐)

> [!TIP]
> **先加载知识库，再扫描代码**。避免遗漏检查项，输出更专业。

**当用户症状明确时**（如"内存暴涨"、"响应慢"）：
```
mcp__java-perf__get_checklist({ symptoms: ["memory"], compact: false })
```

**获取全部反模式清单**（用于通用分析）：
```
mcp__java-perf__get_all_antipatterns({})
```

**用途**：
- 引导后续分析方向
- 输出时引用标准修复方案
- 确保检查项覆盖完整

---

## Phase 1: 🛰️ 雷达扫描 (0 Token)

> [!IMPORTANT]
> **必须先执行雷达扫描**，不要直接搜索文件或使用 grep。

**首选：全项目扫描**
```
mcp__java-perf__radar_scan({ codePath: "./" })
```
返回：全项目嫌疑点列表（P0/P1 分类）

**备选：单文件扫描**
```
mcp__java-perf__scan_source_code({
  code: "文件内容",
  filePath: "xxx.java"
})
```

---

## Phase 2: 🎯 狙击验证 (LSP + 推理)

> [!CAUTION]
> **只跳转到雷达标记的位置**，不要盲目搜索。**关键：使用推理能力验证！**

对每个嫌疑点执行以下推理步骤：

### 步骤 1: 跳转到嫌疑位置
```
mcp__cclsp__find_symbol({ query: "嫌疑方法名" })
```

### 步骤 2: 读取关键代码（限制 50 行）
```
view_file({ path: "x.java", startLine: 100, endLine: 150 })
```

### 步骤 3: 执行推理验证（关键！）

| 嫌疑类型 | 推理问题 | 验证方法 |
|----------|----------|----------|
| **N+1** | "被调用方法是 DAO/RPC 吗？" | 使用 LSP 跳转到被调用方法定义，检查注解 (@Repository, @FeignClient) |
| **ThreadLocal** | "有配对的 remove() 吗？" | 在同一方法内搜索 `.remove()` |
| **锁竞争** | "锁范围有多大？临界区内有 IO 吗？" | 检查 synchronized 块内的代码行数和调用 |
| **无界缓存** | "有 TTL 或 maximumSize 吗？" | 查找 `.expireAfter` 或 `.maximumSize` 配置 |
| **嵌套循环** | "两个集合的规模如何？" | 检查变量来源，推理 N*M 的量级 |

### 步骤 4: 跨文件推理（如果需要）

当 N+1 嫌疑需要确认被调用方法时：
```
mcp__cclsp__find_definition({ symbol: "findByUserId" })
```
然后检查目标方法的：
- 类注解：`@Repository`, `@Mapper`, `@FeignClient`
- 方法注解：`@Query`, `@Select`

---

## Phase 3: 🔬 法医取证 (可选)

仅当需要运行时分析时使用：

| 场景 | 工具 |
|------|------|
| 线程死锁/阻塞 | `mcp__java-perf__analyze_thread_dump({ pid: 12345 })` |
| 字节码锁分析 | `mcp__java-perf__analyze_bytecode({ classPath: "x.class" })` |
| 堆内存分析 | `mcp__java-perf__analyze_heap({ pid: 12345 })` |
| 日志异常归类 | `mcp__java-perf__analyze_log({ logPath: "app.log" })` |
| 引擎状态 | `mcp__java-perf__get_engine_status({})` |

---

## Phase 4: 📊 影响评估 (推理)

> [!IMPORTANT]
> **每个问题必须量化影响**，不要只说"可能有问题"。

### 影响评估模板

对于每个确认的问题，进行以下推理：

1. **放大系数**
   - 循环次数 × 单次耗时 = 总耗时
   - 例：`100 次循环 × 10ms/次 = 1 秒`

2. **内存增长**
   - 对象大小 × 创建频率 × 存活时间
   - 例：`1KB × 1000 次/分钟 × 无 TTL = 1.4GB/天`

3. **并发影响**
   - 锁粒度 × 持有时间 × 请求并发数
   - 例：`方法级锁 × 100ms × 200 并发 = 串行等待`

---

## 症状快速定位

| 症状 | 雷达检测 | 狙击验证 | 影响评估 |
|------|----------|----------|----------|
| **内存** | ThreadLocal, static Map | 检查 remove/TTL | 计算内存增长速率 |
| **CPU** | synchronized, 循环 | 检查锁范围/复杂度 | 估算等待时间 |
| **响应慢** | 循环内调用 | 确认是 DAO/RPC | 计算放大系数 |
| **资源** | Executors.new | 检查是否有界 | 估算峰值线程数 |

---

## 输出格式

每个问题必须包含：

```
📍 **位置**：`文件:行号`
🔍 **根因**：为什么有问题（附推理过程）
📊 **影响**：量化的放大倍数/内存增长/并发瓶颈
🔧 **修复**：可直接应用的代码 Patch
```

---

## 示例

**用户**：系统内存暴涨

**Claude**：

```
# Phase 0: 知识预加载
mcp__java-perf__get_checklist({ symptoms: ["memory"] })
→ 获取内存相关检查项：ThreadLocal、无界缓存、大对象...

# Phase 1: 雷达扫描
mcp__java-perf__radar_scan({ codePath: "./" })
→ 发现 TraceStore.java:45 ThreadLocal 嫌疑

# Phase 2: 狙击验证
view_file({ path: "TraceStore.java", startLine: 40, endLine: 60 })
→ 确认无 finally remove()
→ 推理：线程池复用线程，ThreadLocal 值累积

# Phase 4: 影响评估
→ 每请求 1KB × 1000 QPS × 24 小时 = 最大 86GB/天内存泄漏

# 输出报告
📍 位置：TraceStore.java:45
🔍 根因：ThreadLocal 未清理，线程池复用导致内存累积
📊 影响：每请求泄漏 1KB，1000 QPS 下每天增长 ~86GB
🔧 修复：
​```java
try {
    currentUser.set(user);
    // ...
} finally {
    currentUser.remove();
}
​```
```

---

## 规则覆盖 (v5.2)

| 规则 ID | 检测范围 | 引擎 |
|---------|----------|------|
| N_PLUS_ONE | for / while / foreach 循环内 DAO 调用 | AST |
| NESTED_LOOP | for-for / foreach-foreach / 混合嵌套 | AST |
| SYNC_METHOD | synchronized 方法级锁 | AST |
| THREADLOCAL_LEAK | ThreadLocal.set() 无配对 remove() | AST |
| STREAM_RESOURCE_LEAK | try 块内创建流资源 | AST |
| UNBOUNDED_POOL | Executors.newCachedThreadPool | Regex |
| UNBOUNDED_CACHE | static Map 无 TTL | Regex |
| ... | 更多规则见 `get_all_antipatterns()` | - |
