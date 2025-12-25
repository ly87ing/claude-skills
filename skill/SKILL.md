---
name: java-perf
description: Diagnoses Java performance issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports 响应慢, CPU高, 内存暴涨, 内存溢出, GC频繁, 连接池满, 线程池满, 服务不可用, 超时, 错误率高, 消息积压, or needs 性能排查/性能分析.
---

# Java 性能问题排查 Skill

## 信息收集

### 快速模式
若用户已提供 **代码路径 + 症状**，直接进入分析。

### 引导模式
若信息不足，回复：

```
收到。请告诉我：

**必填**：
- 代码路径：（留空=当前目录）
- 症状：内存暴涨 / CPU高 / 响应慢 / 资源耗尽 / 消息积压 / GC频繁（可多选）

**可选**：
- 日志/Dump路径
```

---

## 分析流程

### Step 1: 一站式诊断（推荐）

> [!IMPORTANT]
> **使用 `diagnose_all` 聚合工具**，一次调用获取所有信息，节省 50%+ Token。

```
mcp__java-perf__diagnose_all({
  symptoms: ["cpu", "slow"],
  priority: "P0",           // 只返回紧急项
  fields: ["diagnosis", "checklist", "patterns"],
  compact: true             // 精简模式
})
```

**症状映射**：
| 用户描述 | MCP 参数 |
|----------|----------|
| 内存暴涨/OOM | `memory` |
| CPU高 | `cpu` |
| 响应慢/超时 | `slow` |
| 连接池满/线程池满 | `resource` |
| 消息积压 | `backlog` |
| GC频繁/STW | `gc` |

---

### Step 2: 代码分析

> [!CAUTION]
> **优先尝试 LSP**，失败后使用 Grep（加 `head_limit: 50`）

诊断工具返回的 `searchPatterns` 包含 LSP 和 Grep 建议。

---

### Step 3: 输出报告

**每个问题必须包含**：
1. 精确位置：`文件:行号`
2. 量化数据：调用次数、放大倍数
3. 可直接应用的修复代码

---

## 示例对话

### 用户
> 帮我排查一下，系统响应很慢，CPU 也很高

### Claude 分析流程

**1. 一次调用获取全部诊断信息**：
```
mcp__java-perf__diagnose_all({
  symptoms: ["cpu", "slow"],
  priority: "P0",
  fields: ["diagnosis", "checklist", "patterns"]
})
```

**返回内容**：
- 组合诊断：锁竞争概率 60%
- P0 检查项：锁与并发、IO阻塞、数据库
- 搜索建议：`synchronized`, `HttpClient`

**2. 代码搜索**（基于返回的 patterns）

**3. 输出报告**
