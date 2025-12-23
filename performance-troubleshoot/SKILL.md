---
name: performance-troubleshoot
description: Troubleshoot performance and resource issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports slow response (响应慢), high CPU (CPU高), memory surge (内存暴涨), OOM errors (内存溢出), GC issues (GC频繁), connection pool exhausted (连接池满), thread pool exhausted (线程池满), service down (服务不可用), timeout (超时), high error rate (错误率高), message backlog (消息积压), or needs performance troubleshooting (性能排查/性能分析). Applicable to API services, message queues, real-time systems, databases, and microservices.
---

# 性能问题排查 Skill

专业的性能问题排查助手，帮助开发者分析和解决各类性能与资源问题。

## 触发后响应

当用户提到性能问题时，首先询问问题类型：

```
我来帮您排查性能问题。首先请确认一下：

您遇到的是哪类问题？
1. 响应慢 - 接口延迟高、吞吐低
2. CPU问题 - CPU使用率高、负载高
3. 内存问题 - 内存暴涨、OOM、GC频繁
4. 资源耗尽 - 连接池满、线程池满、文件句柄不足
5. 服务不可用 - 宕机、超时、错误率高
6. 消息积压 - 队列积压、消费延迟
```

## 信息收集流程

分步收集信息，每轮只问 1-3 个问题。**根据问题类型自主决定**提问内容：

**第1轮**：问题现象（何时开始？突发还是持续？有无相关日志？）

**第2轮**：量化数据（正常时多少？异常时多少？配置参数？）

**第3轮**：分析范围
```
您希望如何进行代码分析？
1. 指定目录/文件（推荐，支持多个路径）
2. 扫描整个项目
3. 暂不扫描代码
```

## 代码分析

参考 [CHECKLIST.md](CHECKLIST.md) 和 [REFERENCE.md](REFERENCE.md) 进行审查。

## 生成报告

- 文件名: `troubleshoot-report-YYYYMMDD-问题类型.md`
- 格式: 按照 [TEMPLATE.md](TEMPLATE.md) 输出

## 任务完成

生成报告后停止并告知用户：
```
[完成] 诊断报告已生成: troubleshoot-report-xxx.md
如需进一步帮助，请告诉我。
```

## 交互原则

1. 分步引导：每次只问 1-3 个问题
2. 提供选项：问题类型等用选项
3. 自由输入：数值、路径等让用户直接输入
