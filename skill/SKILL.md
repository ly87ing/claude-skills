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

**优先尝试 MCP**（如果可用）：
```
mcp__java-perf__diagnose_all({
  symptoms: ["cpu", "slow"],
  priority: "P0",
  compact: true
})
```

**MCP 不可用时，使用内置速查表**：

<details>
<summary>🔧 P0 验证命令速查表（点击展开）</summary>

#### 内存问题 (memory/gc)
| 检查项 | 验证命令 |
|--------|----------|
| 大对象 | `jmap -histo:live PID | head -20` |
| 堆内存 | `jstat -gcutil PID 1000` |
| ThreadLocal 泄露 | 搜索 `ThreadLocal` 未配对 `remove()` |
| 无界缓存 | 搜索 `static.*Map` 无 TTL |

#### CPU 问题 (cpu)
| 检查项 | 验证命令 |
|--------|----------|
| 线程阻塞 | `jstack PID | grep -A 20 "BLOCKED"` |
| 死锁 | `jstack PID | grep "deadlock"` |
| CPU 热点 | `arthas: profiler start/stop` |
| 锁竞争 | `arthas: monitor -c 5 类名 方法名` |

#### 响应慢 (slow)
| 检查项 | 验证命令 |
|--------|----------|
| 方法耗时 | `arthas: trace 类名 方法名` |
| 慢 SQL | `EXPLAIN SELECT ...` |
| N+1 查询 | 开启 SQL 日志，观察重复 SQL |
| 外部调用超时 | 搜索 `timeout/connectTimeout` 配置 |

#### 资源耗尽 (resource)
| 检查项 | 验证命令 |
|--------|----------|
| 线程数 | `arthas: thread -n 10` |
| 文件句柄 | `lsof -p PID | wc -l` |
| 连接池 | `show processlist` (MySQL) |
| 线程池状态 | `jstack PID | grep pool` |

#### 消息积压 (backlog)
| 检查项 | 验证命令 |
|--------|----------|
| 消费者阻塞 | 检查 `@KafkaListener/@RabbitListener` 方法 |
| 队列堆积 | 检查 MQ 控制台 pending 数量 |

</details>

---

### Step 2: 代码分析

> **优先 LSP**，失败后用 Grep（加 `head_limit: 50`）

**搜索关键词**：
| 症状 | LSP 搜索 | Grep 正则 |
|------|----------|-----------|
| memory | `ThreadLocal`, `ConcurrentHashMap` | `static.*Map\|ThreadLocal` |
| cpu | `synchronized`, `ReentrantLock` | `synchronized\|ReentrantLock` |
| slow | `HttpClient`, `Connection` | `HttpClient\|getConnection` |
| resource | `ThreadPoolExecutor`, `DataSource` | `newCachedThreadPool\|DataSource` |

---

### Step 3: 输出报告

每个问题必须包含：
1. **位置**：`文件:行号`
2. **量化**：调用次数、放大倍数
3. **修复代码**：可直接应用

---

## 示例

### 用户
> 系统响应慢，CPU 也很高

### Claude
1. **识别症状**：slow + cpu → 可能是锁竞争(60%)
2. **验证**：`jstack PID | grep BLOCKED`
3. **搜索**：`synchronized`, `ReentrantLock`
4. **定位问题** → 输出修复方案
