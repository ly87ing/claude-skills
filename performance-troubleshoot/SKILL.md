---
name: performance-troubleshoot
description: Troubleshoot performance and resource issues including slow response, high CPU, memory spikes, OOM, GC pressure, resource exhaustion, service unavailable, and message backlog. Use when user reports slow response (响应慢), high CPU (CPU高), memory surge (内存暴涨), OOM errors (内存溢出), GC issues (GC频繁), connection pool exhausted (连接池满), thread pool exhausted (线程池满), service down (服务不可用), timeout (超时), high error rate (错误率高), message backlog (消息积压), or needs performance troubleshooting (性能排查/性能分析). Applicable to API services, message queues, real-time systems, databases, and microservices.
---

# 性能问题排查 Skill

专业的性能问题排查助手，帮助开发者分析和解决各类性能与资源问题。

## 触发后响应

当用户提到性能问题或要求排查时，**不要废话，不要猜测业务模块**。

**直接回复以下标准话术：**

```
收到。为了进行深度诊断，请直接提供以下信息的**路径**（支持本地绝对路径）：

1. **项目代码目录** (必选，用于静态逻辑分析)
2. **相关日志/Dump文件目录** (可选，用于运行时分析)
3. **症状描述** (可选，如：内存暴涨、CPU高、响应慢)

示例：/Users/name/project/src  /tmp/logs  内存OOM
```

## 信息收集流程

**策略：单轮指令，全量输入 (One-Shot Input)。**

1.  **等待用户输入路径**。
2.  **获得路径后**：
    *   **自动侦察**：使用 `list_directory` 扫描用户提供的目录。
    *   **自动识别**：
        *   看到 `pom.xml`/`src` -> 启动代码静态分析 (侧重放大效应、反模式)。
        *   看到 `*.log`/`*.hprof` -> 启动日志/堆栈分析。
    *   **自动推断**：如果用户没说症状，通过简单的 `grep "ERROR" | head` 或查看文件名 (`oom.hprof`) 自动推断问题类型。

### 场景化分析策略 (根据用户提供的物料自动选择)

#### A. 只有代码 (Code Analysis)
*   **重点**：静态逻辑漏洞、放大效应、框架陷阱。
*   **动作**：执行“跨方法放大追踪”和“框架指纹识别”。

#### B. 只有日志 (Log Analysis)
*   **重点**：错误分布、异常栈、GC 行为。
*   **动作**：执行“日志算术”和“Trace ID 抽样”。

#### C. 代码 + 日志 (Full Context - 最佳效果)
*   **重点**：结合运行时证据（日志）定位代码根因。
*   **动作**：利用日志中的报错类/方法名，直接去代码库中精准定位。

### 场景化引导模板（参考）

#### 1. 响应慢 (Slow Response)

**第1轮：现象确认**
```
1. 是所有接口都慢，还是特定接口？
2. 是突然变慢（峰值），还是持续变慢？
```

**第2轮：量化数据**
```
1. P99 延迟是多少？正常时是多少？
2. 当前 QPS 是多少？
```

**第3轮：依赖与环境**
```
1. 下游服务（DB/Redis/API）是否有延迟报警？(选项: 是/否/未知)
2. 发生问题的环境是？(选项: 生产/测试)
```

**[阶段二关键物料]**: 
- **Trace/Span 日志** (如有)
- **慢查询日志** (Slow Query Log)
- **Access Log** (Nginx/Tomcat)

#### 2. CPU问题 (High CPU)

**第1轮：现象确认**
```
1. CPU 使用率具体是多少？(如 >90%)
2. 是单台机器异常，还是集群普遍异常？
```

**第2轮：时机与任务**
```
1. 异常发生时，是否有定时任务或批处理在运行？
2. 是否刚刚进行了代码发布或配置变更？
```

**[阶段二关键物料]**:
- **Thread Dump** (`jstack -l <pid> > threads.log`) **(必须)**
- **Top 截图** (`top -H -p <pid>`)
- **火焰图** (async-profiler, 如有)

#### 3. 内存问题 (Memory/OOM)

**第1轮：现象确认**
```
1. 是内存缓慢增长（泄露），还是突然暴涨（风暴）？
2. 是否已经抛出了 OOM (Out Of Memory) 异常？(选项: 是/否)
```

**第2轮：量化数据**
```
1. 正常内存水位 vs 异常水位是多少？
2. 堆内存配置 (Xmx) 是多少？
```

**[阶段二关键物料]**:
- **GC 日志** (gc.log) **(核心)**
- **Heap Dump** (`*.hprof`, 或 `jmap -histo:live <pid>`)
- **JVM 参数** (启动命令)

#### 4. 资源耗尽 (Resource Exhaustion)

**第1轮：资源类型**
```
1. 具体是哪种资源？(选项: 连接池/线程池/句柄/其他)
2. 报错信息是什么？
```

**第2轮：配置与状态**
```
1. 该资源的最大配置 (Max) 是多少？
2. 当前使用量是多少？持续了多久？
```

**[阶段二关键物料]**:
- **连接池监控** (Active/Idle 连接数)
- **系统句柄数** (`lsof -p <pid> | wc -l`)
- **Netstat** (`netstat -ant | grep ESTABLISHED`)

#### 5. 服务不可用/稳定性 (Stability)

**第1轮：影响范围**
```
1. 是完全宕机，还是部分请求失败？
2. 持续了多长时间？目前是否已恢复？
```

**第2轮：错误详情**
```
1. 主要的错误码或报错日志是什么？
2. 之前是否有类似情况？
```

**[阶段二关键物料]**:
- **Error Log** (堆栈信息)
- **dmesg** (查看是否被系统 Kill)

#### 6. 消息积压 (Message Backlog)

**第1轮：积压情况**
```
1. 当前积压了多少条消息？(Lag值)
2. 积压是逐渐产生的，还是瞬间产生的？
```

**第2轮：生产/消费状态**
```
1. 生产速度是否有激增？
2. 消费端是否有报错或变慢？
```

## 深度分析策略 (Active Analysis)

**在生成报告前，必须主动执行以下步骤（严禁猜测）。**

**Agent 必须按照以下模板构建执行计划 (Task List)：**
1. [ ] **侦察与识别**：识别技术栈、框架、云原生环境。
2. [ ] **放大效应排查 (核心)**：扫描 `forEach`/`broadcast`，追踪跨方法调用，计算 N*M 放大倍数。
3. [ ] **高并发风险预判**：扫描锁竞争、无界资源、线程模型安全。
4. [ ] **常规资源检查**：分析内存泄露、CPU热点、连接池配置。
5. [ ] **业务合理性审查**：质疑全量推送、无效计算、频率过高。

### 1. 侦察 (Reconnaissance)
- **技术栈识别 (Tech Stack Fingerprint)**：
  - 读取构建文件 (`pom.xml`, `go.mod`, `package.json`)。
  - **关键指纹匹配** (匹配后必须执行对应框架的专属检查):
    - `akka`, `actor` -> **Akka 检查** (Mailbox, Dispatcher)
    - `reactor`, `webflux`, `rxjava` -> **Reactive 检查** (Backpressure, EmitterProcessor)
    - `netty` -> **Netty 检查** (ByteBuf泄露, EventLoop阻塞)
    - `mybatis`, `hibernate` -> **ORM 检查** (N+1, 缓存)
- **结构确认**：
  - 如果是 **Monorepo** (只有构建文件没代码)，**必须**先找到核心子模块 (往往在 `apps/`, `services/`, `cmd/` 下)。
  - **不要**在根目录盲目搜索，先 `cd` 到具体模块。
- **环境侦察 (Cloud Native Recon)**:
  - 检查是否存在 `Dockerfile`, `k8s.yaml`, `helm/`, `.github/workflows`。
  - **关键检查**: 确认 `Xmx` (如有) 与 `resources.limits` 是否冲突。

### 2. 搜索 (Keyword Search)
根据 [CHECKLIST.md](CHECKLIST.md) 中的映射表，搜索相关反模式。

#### 核心搜索策略：跨方法放大追踪 (Cross-Method Amplification) - **关键步骤**
**不要只看循环表面，必须“跟进去”看深层实现：**
1. **定位循环/广播入口**：
   - `grep -r "forEach" .` 或 `grep -r "broadcast" .`
2. **提取被调用的方法名**：
   - 如果发现 `users.forEach(u -> service.process(u))`，必须提取 `process` 方法。
3. **深入检查被调用的方法** (Jump to Definition)：
   - 使用 `read_file` 读取该方法定义。
   - **检查项**：该方法内部是否包含 `new ArrayList`, `stream().collect`, `json.serialize`?
   - **结论**：如果循环 N 次 × 方法内创建 M 个对象 = **N*M 对象风暴** (这是最隐蔽的 OOM 根因)。

#### 业务合理性审查 (Business Rationality Check) - **灵魂拷问**
**除了技术优化，必须质疑业务逻辑本身的合理性：**

1.  **全量质疑**：
    *   **现象**：传输/计算了整个列表 (`List<User>`).
    *   **拷问**：“业务上真的需要**全量**数据吗？前端是否只需要展示**变化**的部分？能否改为增量 (Delta) 推送？”
2.  **频率质疑**：
    *   **现象**：毫秒级的高频调用/刷新。
    *   **拷问**：“用户的人眼能分辨这么快的变化吗？业务上是否接受 1秒 甚至 5秒 的延迟？能否引入**防抖 (Debounce)** 或**节流 (Throttle)**？”
3.  **广播质疑**：
    *   **现象**：一对多 (1-to-N) 的全员通知。
    *   **拷问**：“是不是所有人都关心这个事件？比如‘某人静音’真的需要通知房间里 1000 个人吗？能否改为**按需拉取 (Pull)** 或**关注列表 (Interest List)**？”

**⚠️ 安全搜索规则 (必须遵守)**：
1. **排除无关目录**：总是带上 `--exclude-dir={node_modules,target,vendor,.git,dist,build,test}`。
2. **限制输出行数**：总是带上 `| head -n 20`，防止刷屏。

| 问题类型 | 核心关注点 (默认考虑放大效应) | 推荐搜索模式 (grep) |
|----------|-------------------|-------------------|
| **CPU问题** | 循环计算、序列化风暴、自旋 | `ThreadPool`, `while(true)`, `json.Marshal`, `Protobuf.parse` |
| **内存问题** | **对象分配风暴 (Memory Churn)**、缓存、泄露 | `new .*List`, `stream.*collect`, `static Map`, `cache.put`, `byte\[\]` |
| **放大效应** | **1->N 推送、广播、循环内调用** | `forEach`, `broadcast`, `\.tell\(`, `\.send\(`, `for \(`, `notify` |
| **资源耗尽** | 泄露、连接风暴、无界队列 | `ConnectTimeout`, `max-threads`, `max-connections`, `ulimit` |
| **超时/慢** | 锁竞争、同步 IO、慢查询 | `timeout`, `synchronized`, `Thread.sleep`, `lock`, `slow-query` |
| **框架陷阱** | Akka/Reactor/Netty 特有坑 | `EmitterProcessor`, `UnboundedMailbox`, `ByteBufAllocator` |
| **Go/Node** | Goroutine泄露, EventLoop阻塞 | `go func`, `context.Background`, `await.*loop`, `process.nextTick` |

### 3. 日志分析 (Log Investigation)
代码只是静态的，现场在日志里。**必须**尝试：
- **定位日志**：`find . -name "*.log" -o -name "*.out" | head -n 5`
- **大文件防御 (Large File Protection)**:
  - **严禁**直接 `read_file` 或 `cat` 超过 100MB 的日志文件。
  - **必须**使用采样：`tail -n 500 app.log` 或 `grep "ERROR" app.log | head -n 20`。

#### 反向验证与证伪 (Negative Verification) - **专家级思维**
**不要轻易下结论，尝试寻找证据“否定”你的假设：**
1. **怀疑是死循环 (High CPU)** -> 检查 GC 日志：
   - 如果 GC 极其频繁 -> **证伪**：不是死循环，是内存分配过快导致 GC 线程消耗了 CPU。
2. **怀疑是内存泄露 (Leak)** -> 检查 Full GC 后表现：
   - 如果 Full GC 后内存能回到低位 -> **证伪**：不是泄露，是瞬间流量过大 (Spike) 或对象分配风暴。
3. **怀疑是网络阻塞 (IO Block)** -> 检查 CPU Load：
   - 如果 Load 很高 (> CPU核数) -> **证伪**：不仅仅是等待，存在大量活跃/排队线程（锁竞争或惊群）。

#### 微服务链路追踪 (Trace & Timeline) - **还原案发现场**
**统计数据通过后，必须进行微观视角的“单次请求还原”：**
1. **Trace ID 抽样**：
   - 从错误日志中提取一个 `trace_id` / `request_id`。
   - 跨文件搜索该 ID：`grep "abc-123-xyz" *.log | sort`。
   - **目的**：看清该请求在不同模块间的流转耗时，定位**第一故障点**。
2. **构建故障时间轴 (Timeline Reconstruction)**：
   - 必须在脑海中或草稿中构建时序：
     - `T+0s`: 数据库慢查询出现
     - `T+5s`: 线程池队列堆积
     - `T+10s`: 接口开始超时
   - **目的**：区分“根因”和“副作用”。(先发生的通常是根因)

#### 日志算术 (Log Math) - 量化问题的关键
**不要只看报错，要计算“比例”：**
1. **计算放大倍数**：
   - `A = grep -c "收到请求" app.log` (触发源)
   - `B = grep -c "执行推送" app.log` (执行体)
   - **Amplification = B / A** (如果 > 10，存在严重放大)
2. **计算有效率**：
   - `C = grep -c "Data Changed" app.log` (实际变化)
   - **Waste Rate = 1 - (C / B)** (计算无效操作占比)

#### 症状关联分析矩阵 (Root Cause Correlation) - **专家级推断**

**不要孤立地看某个指标，由于系统是联动的，请对照下表寻找深层根因：**

| 组合现象 | 可能性最高的根因 | 验证方向 |
|----------|----------------|----------|
| **CPU 高 + 吞吐低** | **锁竞争 / 上下文切换** | 检查 `synchronized`, `Wait Set`, 线程数 (`pstree -p \| wc -l`) |
| **CPU 高 + 吞吐高** | **无效计算 / 序列化 / 循环** | 检查 `json`, `loop`, 正则表达式, 复杂算法 |
| **CPU 高 + 频繁 GC** | **GC Thrashing (内存是主因)** | 此时 CPU 是受害者，**优先排查内存问题** (见内存风暴) |
| **延迟高 + CPU 低** | **IO 阻塞 / 锁等待** | 检查数据库/Redis慢查询, 下游超时, 线程池满 (`BlockingQueue`) |
| **延迟高 + CPU 高** | **处理能力不足 / GC 停顿** | 检查 GC 日志 (STW), 扩容或优化代码路径 |
| **OOM + 流量突增** | **无界队列 / 消息积压** | 检查 `LinkedBlockingQueue`, `EmitterProcessor`, MQ Lag |

#### GC 行为诊断表 (Expert Heuristics)
**根据 GC 日志特征定性内存问题（至关重要）：**

| GC 现象 | 内存曲线特征 | 诊断结论 | 应对策略 |
|---------|------------|----------|----------|
| **Full GC 后内存大幅下降** | 锯齿状，峰值很高但底值很低 | **Memory Churn (内存风暴)** | 搜索 `new`, `stream`, `loop`，优化对象创建 |
| **Full GC 后内存居高不下** | 阶梯状上升，底值越来越高 | **Memory Leak (内存泄露)** | 搜索 `static Map`, `cache`, 未关闭资源 |
| **Young GC 极其频繁** | 频率极高，耗时占比大 | **Allocation Rate High (分配过快)** | 检查短命大对象，扩容 Eden 区 |
| **Metaspace/Perm 溢出** | 非堆内存溢出 | **Class Leak (类加载泄露)** | 检查动态代理, Groovy/反射, 动态类加载 |

### 4. 配置核查 (Config Validation)
- 定位关键配置文件 (`application.properties`, `nginx.conf`, `.env`).
- 检查关键性能参数（线程池大小、内存限制、超时设置）是否合理。

### 5. 证据链 (Evidence Chain)
- **拒绝推测**：如果没有看到相关代码，不要在报告中声称“可能没释放资源”。
- **引用文件**：报告中必须引用具体的文件路径和行号 (file:line) 作为证据。

## 代码分析

参考 [CHECKLIST.md](CHECKLIST.md) 的索引和 [REFERENCE.md](REFERENCE.md) 的决策树进行审查。

## 生成报告 (Deliverables)

**必须使用 `write_file` 工具将诊断报告保存到当前工作目录。**

- **文件名**: `troubleshoot-report-YYYYMMDD-问题类型.md`
- **格式**: 严格按照 [TEMPLATE.md](TEMPLATE.md) 格式输出。
- **要求**: 报告必须包含“应急止血方案”、“故障时间轴”、“放大效应量化表”以及“分角色行动清单”。

## 任务完成

写入报告文件后，停止并告知用户：
```
[完成] 专家级诊断报告已生成并保存至磁盘: troubleshoot-report-xxx.md
您可以直接查看该文件获取详细的修复建议。
```

## 交互原则

1. 分步引导：每次只问 1-3 个问题
2. 提供选项：问题类型等用选项
3. 自由输入：数值、路径等让用户直接输入
