# 详细排查清单 (Checklist)

> **原则**: 关注代码逻辑与语义，而非仅进行文本搜索。优先使用 LSP 或代码阅读来确认。

## 目录

- [[0] 放大效应追踪](#0-放大效应追踪-amplification)
- [[1] 锁与并发](#1-锁与并发-concurrency--lock)
- [[2] IO 与阻塞](#2-io-与-阻塞-io--blocking)
- [[3] 外部调用](#3-外部调用-external-calls)
- [[4] 资源池管理](#4-资源池管理-resource-pools)
- [[5] 内存与缓存](#5-内存与缓存-memory--cache)
- [[6] 异常处理](#6-异常处理-exception-handling)
- [[7] 启动与配置](#7-启动与配置-startup--config)
- [[8] 日志规范](#8-日志规范-logging)
- [[9] 序列化](#9-序列化-serialization)
- [[10] 正则表达式](#10-正则表达式-regex)
- [[11] 响应式编程](#11-响应式编程-reactive---若适用)
- [[12] 定时任务](#12-定时任务-scheduled-tasks)
- [[13] 数据库交互](#13-数据库交互-database-analysis)
- [[14] Java 特定检查](#14-java-特定检查-java-specific)

---

## [0] 放大效应追踪 (Amplification)
**目标**: 识别 "小输入 -> 大资源消耗" 的代码结构。

- [ ] **流量入口排查**: 根据架构类型锁定对应入口：
    - *同步请求*: Controller, RPC Provider, Gateway
    - *异步消息*: MQ Listener, Actor.receive, Reactive Subscriber
    - *后台任务*: Schedule Job, Batch Task
    - *长连接*: WebSocket, Netty Handler
- [ ] **循环内 IO/计算**: 检查循环体内的数据库查询、RPC 调用或复杂计算。
  - *关注*: `for`/`while`/`stream.forEach` 内部。
- [ ] **集合笛卡尔积**: 检查嵌套循环处理集合的代码 (O(N*M))。
- [ ] **广播风暴**: 检查 "单事件触发全量推送" 的逻辑。
  - *特征*: `Observer` 模式或 `Listener` 中遍历所有用户/连接。
- [ ] **频繁对象创建**: 检查高频/循环路径下的临时对象分配。
  - *关注*: `new ArrayList`, `stream.collect`, JSON 序列化。

## [1] 锁与并发 (Concurrency & Lock)
**目标**: 识别导致线程阻塞或吞吐量下降的并发设计。

- [ ] **锁粒度过大**: 检查 `synchronized` 方法或大块代码段的 `synchronized` 块。
- [ ] **锁竞争**: 分析高频访问的共享资源上的锁使用。
- [ ] **死锁风险**: 检查嵌套锁获取顺序是否一致。
- [ ] **CAS 自旋风险**: 检查 `Atomic` 类的 `do-while` 更新循环是否缺乏退避机制。

## [2] IO 与 阻塞 (IO & Blocking)
**目标**: 识别导致线程池耗尽或响应延迟的 IO 操作。

- [ ] **同步 IO**: 检查 NIO/Netty 线程中是否混入了 JDBC、File IO 或同步 HTTP 调用。
- [ ] **长耗时逻辑**: 检查 Controller/RPC 入口处是否有未异步化的耗时操作。
- [ ] **资源未关闭**: 检查 `InputStream`, `Connection` 等是否在 `finally` 或 `try-with-resources` 中关闭。

## [3] 外部调用 (External Calls)
**目标**: 识别下游依赖导致的稳定性风险。

- [ ] **无超时设置**: 检查 HTTPClient, Dubbo, DB 连接是否设置了 SocketTimeout / ConnectTimeout。
- [ ] **重试风暴**: 检查重试策略是否包含 Backoff (退避) 和 Jitter (抖动)。
- [ ] **同步串行调用**: 检查多个下游调用是否串行执行，是否可改为并行 (CompletableFuture)。

## [4] 资源池管理 (Resource Pools)
**目标**: 识别连接池/线程池的配置风险。

- [ ] **无界线程池**: 检查是否使用了 `Executors.newCachedThreadPool`。
- [ ] **池资源泄露**: 检查从池中获取资源后是否确保归还。
- [ ] **连接数配置**: 检查 DB/Redis 连接池大小是否合理 (避免过小导致等待，过大导致切换)。

## [5] 内存与缓存 (Memory & Cache)
**目标**: 识别内存泄漏和 GC 压力点。

- [ ] **无界缓存**: 检查 `static Map`, `ConcurrentHashMap` 是否由清理机制 (TTL/Size Limit)。
  - *风险*: `List`/`Map` 只增不删。
- [ ] **大对象分配**: 检查一次性加载大文件或全量表数据的逻辑。
- [ ] **ThreadLocal 泄露**: 检查 `ThreadLocal` 是否在请求结束时显式 `remove()`。

## [6] 异常处理 (Exception Handling)
**目标**: 识别错误处理导致的性能或逻辑问题。

- [ ] **异常吞没**: 检查 `catch (Exception e)` 后是否仅打印日志而未抛出或处理。
- [ ] **异常日志爆炸**: 检查是否在高频错误路径上打印了完整堆栈 (Stacktrace)。
- [ ] **高昂异常开销**: 检查是否用异常来控制正常业务流程 (如 `IndexOutOfBounds` 做循环结束条件)。

## [7] 启动与配置 (Startup & Config)
**目标**: 识别初始化阶段的隐患。

- [ ] **Eager 初始化**: 检查是否有耗时资源在类加载时静态初始化 (Static Block)。
- [ ] **配置硬编码**: 检查 URL, 线程数, 超时时间是否硬编码在代码中。

## [8] 日志规范 (Logging)
**目标**: 识别日志导致的性能损耗。

- [ ] **同步日志**: 检查 Log4j2/Logback 是否配置为同步模式 (高并发下列队阻塞)。
- [ ] **字符串拼接**: 检查 `log.info("msg: " + obj)` 这种即使不打印也耗损性能的拼接。
- [ ] **日志级别不当**: 检查是否在生产环境开启了 DEBUG 级别。

## [9] 序列化 (Serialization)
**目标**: 识别序列化相关的性能陷阱。

- [ ] **Java 原生序列化**: 检查是否用于高性能场景 (性能差，体积大)。
- [ ] **大对象序列化**: 检查是否序列化了包含大量无用字段的复杂对象。

## [10] 正则表达式 (Regex)
**目标**: 识别 ReDoS 和正则性能问题。

- [ ] **Catastrophic Backtracking**: 检查嵌套量词 (如 `(a+)+`) 导致的指数级回溯。
- [ ] **反复编译**: 检查 `Pattern.compile` 是否在循环或高频方法中被反复调用 (应使用 static final)。

## [11] 响应式编程 (Reactive - 若适用)
**目标**: 识别 Reactor/RxJava/CompletableFuture 的误用。

- [ ] **阻塞操作**: 检查 `map`/`flatMap` 中是否有 JDBC/RPC 等阻塞调用。
- [ ] **背压丢失**: 检查是否使用了无法处理背压的操作符。

## [12] 定时任务 (Scheduled Tasks)
**目标**: 识别定时任务的堆积风险。

- [ ] **任务堆积**: 检查 `ScheduledExecutorService` 的任务执行时间是否超过了调度间隔。
- [ ] **异常中断**: 检查任务内是否捕获了所有异常 (未捕获异常会导致调度停止)。

## [13] 数据库交互 (Database Analysis)
**目标**: 识别 SQL 相关的性能反模式。

- [ ] **循环查询 (N+1)**: 检查是否在循环中执行 SQL 查询。
- [ ] **全表扫描风险**: 检查查询条件是否包含索引字段。
- [ ] **大数据量 Limit**: 检查深度分页 (`LIMIT 100000, 10`) 带来的性能问题。

## [14] Java 特定检查 (Java Specific)
**目标**: 针对 JDK 特性的深度检查。

- [ ] **Stream 滥用**: 检查简单循环是否被过度封装为 Stream (在极高性能场景下有开销)。
- [ ] **Arrays.asList 坑**: 检查返回的 List 是否支持 add/remove。
- [ ] **BigDecimal 构造**: 检查是否使用了 `new BigDecimal(double)` (精度丢失风险)。
