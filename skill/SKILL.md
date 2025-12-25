---
name: java-perf
description: Diagnoses Java performance issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, and message backlog. Use when user reports 响应慢, CPU高, 内存暴涨, 内存溢出, GC频繁, 连接池满, 线程池满, 超时, 消息积压, or needs 性能排查/性能分析.
---

# Java 性能问题排查 Skill

## 信息收集

若用户已提供 **代码路径 + 症状**，直接进入分析。否则询问：

```
收到。请告诉我：
- 症状：内存暴涨 / CPU高 / 响应慢 / 资源耗尽 / 消息积压 / GC频繁（可多选）
- 代码路径：（留空=当前目录）
```

---

## 分析流程

### Step 1: 获取诊断信息

**优先使用 MCP 工具**（如果可用）：
```
mcp__java-perf__diagnose_all({
  symptoms: ["cpu", "slow"],
  priority: "P0",
  compact: true
})
```

返回：诊断建议 + 检查项 + 搜索关键词

---

### Step 2: 代码分析（重要！）

> [!IMPORTANT]
> **必须使用 `mcp__cclsp__*` 工具进行代码搜索**，不要手动 grep

**使用 cclsp 搜索性能问题代码**：

```
# 1. 搜索符号定义
mcp__cclsp__find_symbol({ query: "synchronized" })
mcp__cclsp__find_symbol({ query: "ThreadLocal" })

# 2. 查找引用
mcp__cclsp__find_references({ file: "xxx.java", line: 123, column: 10 })
```

**搜索关键词**（根据症状）：

| 症状 | cclsp 搜索关键词 |
|------|------------------|
| memory | `ThreadLocal`, `ConcurrentHashMap`, `static Map` |
| cpu | `synchronized`, `ReentrantLock`, `AtomicInteger` |
| slow | `HttpClient`, `RestTemplate`, `@Transactional` |
| resource | `ThreadPoolExecutor`, `DataSource`, `newCachedThreadPool` |
| gc | `new ArrayList`, `StringBuilder`, `stream().` |

**cclsp 不可用时**，使用 grep_search：
```
grep_search({ Query: "synchronized", SearchPath: "./", IsRegex: false })
```

---

### Step 3: 定位问题

对于找到的可疑代码，使用 cclsp 深入分析：

```
# 查看调用链
mcp__cclsp__find_call_hierarchy({ 
  file: "Service.java", 
  line: 50, 
  direction: "incoming"  # 谁调用了这个方法
})

# 查看类型定义
mcp__cclsp__get_hover({ file: "xxx.java", line: 123, column: 10 })
```

---

### Step 4: 输出报告

每个问题必须包含：
1. **位置**：`文件:行号`（用 cclsp 确认）
2. **量化**：调用次数、放大倍数
3. **修复代码**：可直接应用

---

## 内置速查表（MCP 不可用时）

<details>
<summary>🔧 P0 验证命令</summary>

| 症状 | 验证命令 |
|------|----------|
| 内存 | `jmap -histo:live PID | head -20` |
| CPU | `jstack PID | grep -A 20 "BLOCKED"` |
| 慢 | `arthas: trace 类名 方法名` |
| 资源 | `lsof -p PID | wc -l` |

</details>

---

## 示例

### 用户
> 系统响应慢，CPU 也很高

### Claude 分析流程

1. **获取诊断**：
   ```
   mcp__java-perf__diagnose_all({ symptoms: ["cpu", "slow"], priority: "P0" })
   ```

2. **搜索可疑代码**：
   ```
   mcp__cclsp__find_symbol({ query: "synchronized" })
   mcp__cclsp__find_symbol({ query: "ReentrantLock" })
   ```

3. **分析调用链**：
   ```
   mcp__cclsp__find_call_hierarchy({ file: "锁方法.java", line: 行号 })
   ```

4. **输出修复方案**
