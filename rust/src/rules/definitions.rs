//! 规则定义
//!
//! 所有规则在此集中定义，确保单一数据源

use super::{Category, DetectorType, RuleDefinition, Severity};

/// 获取所有规则定义
pub fn all_rules() -> Vec<RuleDefinition> {
    let mut rules = Vec::new();

    // === 性能规则 ===
    rules.extend(performance_rules());

    // === 并发规则 ===
    rules.extend(concurrency_rules());

    // === 内存规则 ===
    rules.extend(memory_rules());

    // === Spring 规则 ===
    rules.extend(spring_rules());

    // === 响应式规则 ===
    rules.extend(reactive_rules());

    // === 资源规则 ===
    rules.extend(resource_rules());

    // === 异常处理规则 ===
    rules.extend(exception_rules());

    // === 数据库规则 ===
    rules.extend(database_rules());

    // === GraalVM 规则 ===
    rules.extend(graalvm_rules());

    // === 配置规则 ===
    rules.extend(config_rules());

    rules
}

// ============================================================================
// 性能规则
// ============================================================================

fn performance_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "N_PLUS_ONE",
            category: Category::Performance,
            severity: Severity::P0,
            description: "循环内数据库/RPC 调用 (N+1 问题)",
            rationale: "循环 100 次 × 每次 10ms = 1 秒响应时间。这是最常见的性能问题。",
            fix_suggestion: "使用批量查询替代循环查询，如 findAllByIdIn(ids)",
            detector: DetectorType::Ast {
                query: r#"
                    [
                        (for_statement body: (block (expression_statement (method_invocation name: (identifier) @method_name))))
                        (enhanced_for_statement body: (block (expression_statement (method_invocation name: (identifier) @method_name))))
                        (while_statement body: (block (expression_statement (method_invocation name: (identifier) @method_name))))
                    ]
                "#,
                handler: Some("N_PLUS_ONE"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "NESTED_LOOP",
            category: Category::Performance,
            severity: Severity::P0,
            description: "嵌套循环 O(N×M) 复杂度",
            rationale: "100×100 = 10000 次迭代。如果内部有任何开销，性能会急剧下降。",
            fix_suggestion: "使用 Map/Set 将复杂度降为 O(N+M)",
            detector: DetectorType::Ast {
                query: r#"
                    [
                        (for_statement body: (block (for_statement) @inner_loop))
                        (for_statement body: (block (enhanced_for_statement) @inner_loop))
                        (enhanced_for_statement body: (block (for_statement) @inner_loop))
                        (enhanced_for_statement body: (block (enhanced_for_statement) @inner_loop))
                    ]
                "#,
                handler: Some("NESTED_LOOP"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "OBJECT_IN_LOOP",
            category: Category::Performance,
            severity: Severity::P1,
            description: "循环内频繁创建对象",
            rationale: "频繁 new 导致 GC 压力，特别是在热点路径上。",
            fix_suggestion: "考虑对象池或将对象创建移到循环外",
            detector: DetectorType::Ast {
                query: r#"
                    [
                        (for_statement body: (block (local_variable_declaration (variable_declarator value: (object_creation_expression) @creation))))
                        (enhanced_for_statement body: (block (local_variable_declaration (variable_declarator value: (object_creation_expression) @creation))))
                    ]
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "STRING_CONCAT_LOOP",
            category: Category::Performance,
            severity: Severity::P1,
            description: "循环内字符串拼接",
            rationale: "每次 += 都创建新 String 对象，O(N²) 复杂度。",
            fix_suggestion: "使用 StringBuilder",
            detector: DetectorType::Ast {
                query: r#"
                    [
                        (for_statement body: (block (expression_statement (assignment_expression left: (_) @var operator: "+=" right: (_) @value)) @assign))
                        (enhanced_for_statement body: (block (expression_statement (assignment_expression left: (_) @var operator: "+=" right: (_) @value)) @assign))
                    ]
                "#,
                handler: Some("STRING_CONCAT_LOOP"),
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 并发规则
// ============================================================================

fn concurrency_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "SYNC_METHOD",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "方法级 synchronized 锁粒度过大",
            rationale: "大锁让并发变串行，严重影响吞吐量。",
            fix_suggestion: "细化锁粒度到代码块级别，或使用读写锁",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers) @mods
                        name: (identifier) @method_name
                        body: (block) @body
                    )
                "#,
                handler: Some("SYNC_METHOD"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SYNC_BLOCK",
            category: Category::Concurrency,
            severity: Severity::P1,
            description: "synchronized 代码块",
            rationale: "请确保锁范围最小化。JDK 21+ Virtual Threads 下会导致 Carrier Thread Pinning。",
            fix_suggestion: "考虑使用 ReentrantLock 或减小锁范围",
            detector: DetectorType::Ast {
                query: r#"
                    (synchronized_statement
                        (parenthesized_expression) @lock_obj
                        body: (block) @body
                    ) @sync
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SLEEP_IN_LOCK",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "持锁时调用 Thread.sleep()",
            rationale: "持锁睡眠导致其他线程长时间阻塞，严重影响并发性能。",
            fix_suggestion: "将 sleep 移出 synchronized 块，或使用 wait/notify",
            detector: DetectorType::Ast {
                query: r#"
                    (synchronized_statement
                        body: (block
                            (expression_statement
                                (method_invocation
                                    object: (identifier) @obj
                                    name: (identifier) @method
                                    (#eq? @obj "Thread")
                                    (#eq? @method "sleep")
                                )
                            )
                        )
                    ) @sync
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "LOCK_METHOD_CALL",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "持锁时调用外部方法",
            rationale: "在 synchronized 块内调用外部方法可能导致死锁。",
            fix_suggestion: "尽量减少持锁时的操作，避免调用可能阻塞的方法",
            detector: DetectorType::Ast {
                query: r#"
                    (synchronized_statement
                        body: (block
                            (expression_statement
                                (method_invocation
                                    object: (identifier) @receiver
                                    name: (identifier) @method
                                )
                            ) @call
                        )
                    ) @sync
                "#,
                handler: Some("LOCK_METHOD_CALL"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "FUTURE_GET_NO_TIMEOUT",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "Future.get() 无超时参数",
            rationale: "无超时的 get() 可能导致线程永久阻塞。",
            fix_suggestion: "使用 future.get(timeout, TimeUnit.SECONDS)",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        arguments: (argument_list) @args
                        (#eq? @method_name "get")
                    ) @call
                "#,
                handler: Some("FUTURE_GET_NO_TIMEOUT"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "AWAIT_NO_TIMEOUT",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "await()/acquire() 无超时参数",
            rationale: "CountDownLatch.await() 或 Semaphore.acquire() 无超时可能永久等待。",
            fix_suggestion: "使用 await(timeout, unit) 或 tryAcquire(timeout, unit)",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        arguments: (argument_list) @args
                        (#match? @method_name "^(await|acquire)$")
                    ) @call
                "#,
                handler: Some("AWAIT_NO_TIMEOUT"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "COMPLETABLE_JOIN",
            category: Category::Concurrency,
            severity: Severity::P1,
            description: "CompletableFuture.join() 无超时",
            rationale: "join() 永久阻塞，无法设置超时。",
            fix_suggestion: "使用 orTimeout() 或 completeOnTimeout()",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#eq? @method_name "join")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "UNBOUNDED_POOL",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "无界线程池 Executors.newCachedThreadPool()",
            rationale: "无界池遇到流量洪峰会无限创建线程，导致 OOM。",
            fix_suggestion: "使用 ThreadPoolExecutor 配置有界队列",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "Executors")
                        (#match? @method_name "^(newCachedThreadPool|newScheduledThreadPool|newSingleThreadExecutor)$")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "ATOMIC_SPIN",
            category: Category::Concurrency,
            severity: Severity::P1,
            description: "高竞争下的 Atomic 自旋",
            rationale: "高并发下 AtomicInteger.incrementAndGet() 会导致大量 CAS 重试。",
            fix_suggestion: "考虑使用 LongAdder 替代",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @obj
                        name: (identifier) @method
                        (#match? @method "^(incrementAndGet|decrementAndGet|addAndGet|getAndIncrement)$")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        // === 新增并发规则 ===
        RuleDefinition {
            id: "DOUBLE_CHECKED_LOCKING",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "双重检查锁定模式 (DCL) 未使用 volatile",
            rationale: "没有 volatile 的 DCL 在多线程下可能看到部分构造的对象。",
            fix_suggestion: "确保实例字段使用 volatile 修饰，或使用静态内部类/枚举实现单例",
            detector: DetectorType::Ast {
                query: r#"
                    (if_statement
                        condition: (parenthesized_expression
                            (binary_expression
                                left: (identifier) @instance
                                operator: "=="
                                right: (null_literal)
                            )
                        )
                        consequence: (block
                            (synchronized_statement
                                body: (block
                                    (if_statement) @inner_if
                                )
                            )
                        )
                    ) @dcl
                "#,
                handler: Some("DOUBLE_CHECKED_LOCKING"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "NON_ATOMIC_COMPOUND",
            category: Category::Concurrency,
            severity: Severity::P0,
            description: "非原子复合操作 (check-then-act)",
            rationale: "if (map.containsKey(k)) { map.get(k) } 不是原子操作，存在竞态条件。",
            fix_suggestion: "使用 computeIfAbsent/putIfAbsent 等原子方法",
            detector: DetectorType::Ast {
                query: r#"
                    (if_statement
                        condition: (parenthesized_expression
                            (method_invocation
                                name: (identifier) @check_method
                                (#match? @check_method "^(containsKey|contains|isEmpty)$")
                            )
                        )
                        consequence: (block
                            (expression_statement
                                (method_invocation
                                    name: (identifier) @action_method
                                    (#match? @action_method "^(get|put|add|remove)$")
                                )
                            )
                        )
                    ) @check_then_act
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 内存规则
// ============================================================================

fn memory_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "THREADLOCAL_LEAK",
            category: Category::Memory,
            severity: Severity::P0,
            description: "ThreadLocal 未调用 remove()",
            rationale: "线程池复用线程，ThreadLocal 不清理会导致内存泄露和数据污染。",
            fix_suggestion: "在 finally 块中调用 remove()",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        body: (block
                            (expression_statement
                                (method_invocation
                                    object: (identifier) @tl_var
                                    name: (identifier) @method_name
                                    (#eq? @method_name "set")
                                )
                            )
                        )
                    ) @method
                "#,
                handler: Some("THREADLOCAL_LEAK"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "STATIC_COLLECTION",
            category: Category::Memory,
            severity: Severity::P0,
            description: "静态集合无大小限制",
            rationale: "只增不删的 static Map/List 是内存泄露。",
            fix_suggestion: "使用 Caffeine/Guava Cache 配置 maximumSize 和 TTL",
            detector: DetectorType::Ast {
                query: r#"
                    (field_declaration
                        (modifiers) @mods
                        type: (_) @type_name
                        declarator: (variable_declarator
                            name: (identifier) @field_name
                            value: (object_creation_expression) @init
                        )
                    ) @field
                "#,
                handler: Some("STATIC_COLLECTION"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "FINALIZE_OVERRIDE",
            category: Category::Memory,
            severity: Severity::P0,
            description: "重写 finalize() 方法",
            rationale: "finalize() 已废弃，会导致对象存活更长时间，影响 GC。",
            fix_suggestion: "使用 Cleaner 或 try-with-resources",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers) @mods
                        name: (identifier) @method_name
                        (#eq? @method_name "finalize")
                    ) @method
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "STRING_INTERN",
            category: Category::Memory,
            severity: Severity::P1,
            description: "使用 String.intern()",
            rationale: "过度使用 intern() 可能导致字符串常量池膨胀。",
            fix_suggestion: "检查是否真的需要 intern()，考虑使用 HashMap 替代",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#eq? @method_name "intern")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "LARGE_ARRAY",
            category: Category::Memory,
            severity: Severity::P1,
            description: "大数组分配 (>1MB)",
            rationale: "大对象直接进入老年代，可能触发 Full GC。",
            fix_suggestion: "考虑使用对象池或分块处理",
            detector: DetectorType::Ast {
                query: r#"
                    (array_creation_expression
                        type: (integral_type) @type_name
                        dimensions: (dimensions_expr
                            (decimal_integer_literal) @size
                        )
                    ) @creation
                "#,
                handler: Some("LARGE_ARRAY"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SOFT_REFERENCE",
            category: Category::Memory,
            severity: Severity::P1,
            description: "SoftReference 使用",
            rationale: "SoftReference 在内存不足时才回收，可能导致意外的内存占用。",
            fix_suggestion: "确保理解 SoftReference 的行为，考虑使用 WeakReference 或显式缓存",
            detector: DetectorType::Ast {
                query: r#"
                    (object_creation_expression
                        type: (generic_type
                            (type_identifier) @type_name
                            (#eq? @type_name "SoftReference")
                        )
                    ) @creation
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// Spring 规则
// ============================================================================

fn spring_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "ASYNC_DEFAULT_POOL",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Async 未指定线程池",
            rationale: "默认使用 SimpleAsyncTaskExecutor，每次创建新线程。",
            fix_suggestion: "配置自定义 Executor 并在 @Async 中指定",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers
                            (marker_annotation
                                name: (identifier) @ann_name
                                (#eq? @ann_name "Async")
                            )
                        )
                    ) @method
                "#,
                handler: Some("ASYNC_DEFAULT_POOL"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SCHEDULED_FIXED_RATE",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Scheduled(fixedRate) 任务堆积风险",
            rationale: "如果任务执行时间超过间隔，会导致任务堆积。",
            fix_suggestion: "改用 fixedDelay 或添加分布式锁",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers
                            (annotation
                                name: (identifier) @ann_name
                                arguments: (annotation_argument_list) @args
                                (#eq? @ann_name "Scheduled")
                            )
                        )
                    ) @method
                "#,
                handler: Some("SCHEDULED_FIXED_RATE"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "AUTOWIRED_FIELD",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Autowired 字段注入",
            rationale: "字段注入不利于测试和不可变性。",
            fix_suggestion: "改用构造器注入",
            detector: DetectorType::Ast {
                query: r#"
                    (field_declaration
                        (modifiers
                            (marker_annotation
                                name: (identifier) @ann_name
                                (#eq? @ann_name "Autowired")
                            )
                        )
                    ) @field
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "CACHEABLE_NO_KEY",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Cacheable 未指定 key",
            rationale: "默认 key 生成策略可能导致缓存冲突。",
            fix_suggestion: "显式指定 key 或 keyGenerator",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers
                            (annotation
                                name: (identifier) @ann_name
                                (#eq? @ann_name "Cacheable")
                            )
                        )
                    ) @method
                "#,
                handler: Some("CACHEABLE_NO_KEY"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "TRANSACTIONAL_REQUIRES_NEW",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Transactional(REQUIRES_NEW) 嵌套事务",
            rationale: "嵌套事务可能导致死锁和资源争用。",
            fix_suggestion: "仔细检查是否真的需要新事务，考虑使用 REQUIRED 传播",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers
                            (annotation
                                name: (identifier) @ann_name
                                arguments: (annotation_argument_list
                                    (element_value_pair
                                        key: (identifier) @prop_name
                                        value: (_) @prop_value
                                    )
                                )
                                (#eq? @ann_name "Transactional")
                            )
                        )
                    ) @method
                "#,
                handler: Some("TRANSACTIONAL_REQUIRES_NEW"),
            },
            enabled_by_default: true,
        },
        // === 新增 Spring 规则 ===
        RuleDefinition {
            id: "TRANSACTION_SELF_CALL",
            category: Category::Spring,
            severity: Severity::P0,
            description: "@Transactional 方法的自调用",
            rationale: "同一个类中的方法调用不经过代理，@Transactional 注解失效。",
            fix_suggestion: "将方法移到另一个 Bean，或使用 AopContext.currentProxy()",
            detector: DetectorType::Ast {
                query: r#"
                    (method_declaration
                        (modifiers
                            (annotation
                                name: (identifier) @ann_name
                                (#eq? @ann_name "Transactional")
                            )
                        )
                        body: (block
                            (expression_statement
                                (method_invocation
                                    name: (identifier) @called_method
                                )
                            )
                        )
                    ) @method
                "#,
                handler: Some("TRANSACTION_SELF_CALL"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "LAZY_INIT_CIRCULAR",
            category: Category::Spring,
            severity: Severity::P1,
            description: "@Lazy 可能隐藏循环依赖",
            rationale: "@Lazy 只是延迟问题，运行时仍可能出现循环依赖错误。",
            fix_suggestion: "重构代码消除循环依赖",
            detector: DetectorType::Ast {
                query: r#"
                    (field_declaration
                        (modifiers
                            (marker_annotation
                                name: (identifier) @ann1
                                (#eq? @ann1 "Lazy")
                            )
                            (marker_annotation
                                name: (identifier) @ann2
                                (#eq? @ann2 "Autowired")
                            )
                        )
                    ) @field
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 响应式规则
// ============================================================================

fn reactive_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "FLUX_BLOCK",
            category: Category::Reactive,
            severity: Severity::P0,
            description: "Flux/Mono.block() 阻塞调用",
            rationale: "在响应式线程中调用 block() 会导致死锁。",
            fix_suggestion: "使用 subscribeOn 切换到弹性线程池",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#match? @method_name "^(block|blockFirst|blockLast)$")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SUBSCRIBE_NO_ERROR",
            category: Category::Reactive,
            severity: Severity::P1,
            description: "subscribe() 未处理 error",
            rationale: "未处理的错误会被静默忽略。",
            fix_suggestion: "添加 error consumer: subscribe(onNext, onError)",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        arguments: (argument_list) @args
                        (#eq? @method_name "subscribe")
                    ) @call
                "#,
                handler: Some("SUBSCRIBE_NO_ERROR"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "FLUX_COLLECT_LIST",
            category: Category::Reactive,
            severity: Severity::P1,
            description: "collectList() 可能导致 OOM",
            rationale: "无界收集可能消耗大量内存。",
            fix_suggestion: "使用 buffer(n) 或 window(n) 限制大小",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#eq? @method_name "collectList")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "PARALLEL_NO_RUN_ON",
            category: Category::Reactive,
            severity: Severity::P1,
            description: "parallel() 未指定 runOn",
            rationale: "默认在调用线程执行，可能阻塞 event loop。",
            fix_suggestion: "添加 .runOn(Schedulers.parallel())",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#eq? @method_name "parallel")
                    ) @call
                "#,
                handler: Some("PARALLEL_NO_RUN_ON"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "EMITTER_UNBOUNDED",
            category: Category::Reactive,
            severity: Severity::P0,
            description: "EmitterProcessor.create() 无界",
            rationale: "无界 EmitterProcessor 可能导致背压失效和 OOM。",
            fix_suggestion: "使用 Sinks.many().multicast().onBackpressureBuffer(n)",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        arguments: (argument_list) @args
                        (#eq? @class_name "EmitterProcessor")
                        (#eq? @method_name "create")
                    ) @call
                "#,
                handler: Some("EMITTER_UNBOUNDED"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "SINKS_MANY",
            category: Category::Reactive,
            severity: Severity::P1,
            description: "Sinks.many() 使用",
            rationale: "检查是否配置了适当的背压策略。",
            fix_suggestion: "确保使用 onBackpressureBuffer(n) 或其他背压策略",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "Sinks")
                        (#eq? @method_name "many")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 资源规则
// ============================================================================

fn resource_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "STREAM_RESOURCE_LEAK",
            category: Category::Resource,
            severity: Severity::P0,
            description: "Stream 资源未关闭",
            rationale: "Files.lines() 等返回的 Stream 需要关闭。",
            fix_suggestion: "使用 try-with-resources",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "Files")
                        (#match? @method_name "^(lines|list|walk|find)$")
                    ) @call
                "#,
                handler: Some("STREAM_RESOURCE_LEAK"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "BLOCKING_IO",
            category: Category::Resource,
            severity: Severity::P1,
            description: "同步阻塞 IO",
            rationale: "FileInputStream 等阻塞 IO 在高并发下影响性能。",
            fix_suggestion: "考虑使用 NIO 或异步 IO",
            detector: DetectorType::Ast {
                query: r#"
                    (object_creation_expression
                        type: (type_identifier) @type_name
                        (#match? @type_name "^(FileInputStream|FileOutputStream|FileReader|FileWriter)$")
                    ) @creation
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "DATASOURCE_NO_POOL",
            category: Category::Resource,
            severity: Severity::P1,
            description: "DriverManager 直连数据库",
            rationale: "不使用连接池会导致频繁创建/销毁连接。",
            fix_suggestion: "使用 HikariCP 等连接池",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "DriverManager")
                        (#eq? @method_name "getConnection")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "CACHE_NO_EXPIRE",
            category: Category::Resource,
            severity: Severity::P1,
            description: "缓存可能无过期配置",
            rationale: "无过期的缓存会无限增长。",
            fix_suggestion: "配置 maximumSize 和 expireAfterWrite",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#match? @class_name "^(Caffeine|CacheBuilder)$")
                        (#eq? @method_name "newBuilder")
                    ) @call
                "#,
                handler: Some("CACHE_NO_EXPIRE"),
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 异常处理规则
// ============================================================================

fn exception_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "EMPTY_CATCH",
            category: Category::Exception,
            severity: Severity::P0,
            description: "空 catch 块",
            rationale: "吞掉异常会导致问题难以排查。",
            fix_suggestion: "至少记录日志，或重新抛出",
            detector: DetectorType::Ast {
                query: r#"
                    (catch_clause
                        body: (block) @body
                    ) @catch
                "#,
                handler: Some("EMPTY_CATCH"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "LOG_STRING_CONCAT",
            category: Category::Exception,
            severity: Severity::P1,
            description: "日志使用字符串拼接",
            rationale: "即使日志级别不匹配，拼接仍会执行。",
            fix_suggestion: "使用占位符: log.info(\"x={}\", x)",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @logger
                        name: (identifier) @method_name
                        arguments: (argument_list
                            (binary_expression
                                operator: "+"
                            )
                        )
                        (#match? @method_name "^(debug|info|warn|error|trace)$")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 数据库规则
// ============================================================================

fn database_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "SELECT_STAR",
            category: Category::Database,
            severity: Severity::P1,
            description: "SELECT * 查询",
            rationale: "查询不需要的字段浪费带宽和内存。",
            fix_suggestion: "明确指定需要的字段",
            detector: DetectorType::Regex {
                pattern: r#"["']SELECT\s+\*\s+FROM"#,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "LIKE_LEADING_WILDCARD",
            category: Category::Database,
            severity: Severity::P0,
            description: "LIKE '%xxx' 前导通配符",
            rationale: "前导通配符导致全表扫描，无法使用索引。",
            fix_suggestion: "改用全文索引，或重新设计查询",
            detector: DetectorType::Regex {
                pattern: r#"LIKE\s+['"]%"#,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// GraalVM 规则
// ============================================================================

fn graalvm_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "GRAALVM_CLASS_FORNAME",
            category: Category::GraalVM,
            severity: Severity::P1,
            description: "Class.forName() 反射调用",
            rationale: "GraalVM Native Image 需要在 reflect-config.json 中配置。",
            fix_suggestion: "添加到 reflect-config.json 或使用 @RegisterForReflection",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "Class")
                        (#eq? @method_name "forName")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "GRAALVM_METHOD_INVOKE",
            category: Category::GraalVM,
            severity: Severity::P1,
            description: "Method.invoke() 反射调用",
            rationale: "GraalVM Native Image 需要配置反射。",
            fix_suggestion: "添加到 reflect-config.json",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        name: (identifier) @method_name
                        (#eq? @method_name "invoke")
                    ) @call
                "#,
                handler: Some("GRAALVM_METHOD_INVOKE"),
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "GRAALVM_PROXY",
            category: Category::GraalVM,
            severity: Severity::P1,
            description: "动态代理 Proxy.newProxyInstance()",
            rationale: "GraalVM Native Image 需要配置动态代理。",
            fix_suggestion: "添加到 proxy-config.json",
            detector: DetectorType::Ast {
                query: r#"
                    (method_invocation
                        object: (identifier) @class_name
                        name: (identifier) @method_name
                        (#eq? @class_name "Proxy")
                        (#eq? @method_name "newProxyInstance")
                    ) @call
                "#,
                handler: None,
            },
            enabled_by_default: true,
        },
    ]
}

// ============================================================================
// 配置规则
// ============================================================================

fn config_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            id: "DB_POOL_SMALL",
            category: Category::Config,
            severity: Severity::P1,
            description: "数据库连接池过小",
            rationale: "连接池太小会导致请求排队等待连接。",
            fix_suggestion: "根据并发量调整 maximum-pool-size (建议 >= 10)",
            detector: DetectorType::Config {
                key: "spring.datasource.hikari.maximum-pool-size",
                simple_key: "maximum-pool-size",
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "TOMCAT_THREADS_LOW",
            category: Category::Config,
            severity: Severity::P1,
            description: "Tomcat 最大线程数过低",
            rationale: "线程数太少会限制并发处理能力。",
            fix_suggestion: "默认 200，根据负载调整",
            detector: DetectorType::Config {
                key: "server.tomcat.max-threads",
                simple_key: "max-threads",
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "JPA_OPEN_IN_VIEW",
            category: Category::Config,
            severity: Severity::P0,
            description: "JPA open-in-view 已启用",
            rationale: "导致延迟加载在视图层执行，可能产生 N+1 问题。",
            fix_suggestion: "设置 spring.jpa.open-in-view=false",
            detector: DetectorType::Config {
                key: "spring.jpa.open-in-view",
                simple_key: "open-in-view",
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "JPA_SHOW_SQL",
            category: Category::Config,
            severity: Severity::P1,
            description: "JPA show-sql 已启用",
            rationale: "生产环境打印 SQL 影响性能。",
            fix_suggestion: "生产环境设置 spring.jpa.show-sql=false",
            detector: DetectorType::Config {
                key: "spring.jpa.show-sql",
                simple_key: "show-sql",
            },
            enabled_by_default: true,
        },
        RuleDefinition {
            id: "DEBUG_LOG_LEVEL",
            category: Category::Config,
            severity: Severity::P1,
            description: "根日志级别为 DEBUG",
            rationale: "DEBUG 日志输出量大，影响性能。",
            fix_suggestion: "生产环境使用 INFO 或更高级别",
            detector: DetectorType::Config {
                key: "logging.level.root",
                simple_key: "level",
            },
            enabled_by_default: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_rules_have_unique_ids() {
        let rules = all_rules();
        let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for rule in &rules {
            assert!(ids.insert(rule.id), "Duplicate rule ID: {}", rule.id);
        }
    }

    #[test]
    fn test_all_rules_have_descriptions() {
        let rules = all_rules();

        for rule in &rules {
            assert!(!rule.description.is_empty(), "Rule {} has empty description", rule.id);
            assert!(!rule.rationale.is_empty(), "Rule {} has empty rationale", rule.id);
            assert!(!rule.fix_suggestion.is_empty(), "Rule {} has empty fix_suggestion", rule.id);
        }
    }

    #[test]
    fn test_rule_count() {
        let rules = all_rules();
        // 确保有足够多的规则
        assert!(rules.len() >= 40, "Should have at least 40 rules, got {}", rules.len());
    }
}
