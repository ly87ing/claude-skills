/**
 * Checklist 数据 - 根据症状返回相关检查项
 */

export interface ChecklistItem {
    id: string;
    title: string;
    items: string[];
}

export const CHECKLIST_DATA: Record<string, ChecklistItem> = {
    '0': {
        id: '0',
        title: '放大效应追踪',
        items: [
            '流量入口排查（Controller, MQ Listener, Schedule Job, WebSocket）',
            '循环内 IO/计算（for/while/stream 内的 DB 查询、RPC、复杂计算）',
            '集合笛卡尔积（嵌套循环 O(N*M)）',
            '广播风暴（单事件触发全量推送）',
            '频繁对象创建（循环内 new 对象、stream.collect）'
        ]
    },
    '1': {
        id: '1',
        title: '锁与并发',
        items: [
            '锁粒度过大（synchronized 方法或大代码块）',
            '锁竞争（高频访问的共享资源）',
            '死锁风险（嵌套锁获取顺序不一致）',
            'CAS 自旋（Atomic 的 do-while 无退避）'
        ]
    },
    '2': {
        id: '2',
        title: 'IO 与阻塞',
        items: [
            '同步 IO（NIO/Netty 线程中混入 JDBC/File IO/同步 HTTP）',
            '长耗时逻辑（Controller 入口未异步化的耗时操作）',
            '资源未关闭（InputStream/Connection 未在 finally 或 try-with-resources 关闭）'
        ]
    },
    '3': {
        id: '3',
        title: '外部调用',
        items: [
            '无超时设置（HTTPClient, Dubbo, DB 连接）',
            '重试风暴（无 Backoff 和 Jitter）',
            '同步串行调用（多下游串行，可改 CompletableFuture 并行）'
        ]
    },
    '4': {
        id: '4',
        title: '资源池管理',
        items: [
            '无界线程池（Executors.newCachedThreadPool）',
            '池资源泄露（获取后未归还）',
            '连接数配置不当（过小等待/过大切换）'
        ]
    },
    '5': {
        id: '5',
        title: '内存与缓存',
        items: [
            '无界缓存（static Map 无 TTL/Size 限制，只增不删）',
            '大对象分配（一次性加载大文件/全量表）',
            'ThreadLocal 泄露（请求结束未 remove()）'
        ]
    },
    '6': {
        id: '6',
        title: '异常处理',
        items: [
            '异常吞没（catch 后仅打印，未抛出/处理）',
            '异常日志爆炸（高频错误路径打印完整堆栈）',
            '异常控制流程（用异常做正常业务流程控制）'
        ]
    },
    '10': {
        id: '10',
        title: '正则表达式',
        items: [
            'Catastrophic Backtracking（嵌套量词 (a+)+ 指数回溯）',
            '反复编译（Pattern.compile 在循环/高频方法中被反复调用）'
        ]
    },
    '11': {
        id: '11',
        title: '响应式编程',
        items: [
            '阻塞操作（map/flatMap 中有 JDBC/RPC 阻塞）',
            '背压丢失（无法处理背压的操作符）'
        ]
    },
    '12': {
        id: '12',
        title: '定时任务',
        items: [
            '任务堆积（执行时间超过调度间隔）',
            '异常中断（未捕获异常导致调度停止）'
        ]
    }
};

// 症状到章节的映射
export const SYMPTOM_TO_SECTIONS: Record<string, string[]> = {
    'memory': ['0', '5', '6'],
    'cpu': ['0', '1', '10'],
    'slow': ['2', '3', '1'],
    'resource': ['4', '5'],
    'backlog': ['0', '11', '12'],
    'gc': ['5', '0']
};

// 快速诊断表
export const QUICK_DIAGNOSIS: Record<string, { causes: string[], patterns: string[] }> = {
    'memory': {
        causes: ['对象创建风暴', '资源泄露', '无界缓存'],
        patterns: ['对象池', '生命周期管理', 'TTL/Size 限制']
    },
    'cpu': {
        causes: ['死循环', '正则回溯', '锁竞争', 'CAS 自旋'],
        patterns: ['算法优化', '锁分段', '退避机制']
    },
    'slow': {
        causes: ['IO阻塞', '锁竞争', '下游慢', '串行调用'],
        patterns: ['异步化', '熔断', '缓存', '并行调用']
    },
    'resource': {
        causes: ['连接池/线程池满', '句柄泄露', '无界队列'],
        patterns: ['资源复用', '背压', '有界队列']
    },
    'backlog': {
        causes: ['消费慢', '突发流量', '处理能力不足'],
        patterns: ['批量消费', '并行消费', '限流']
    }
};

// 反模式速查
export const ANTI_PATTERNS = [
    { name: '锁内IO', bad: 'synchronized { httpClient.get() }', good: '锁外获取，锁内只写' },
    { name: '循环创建对象', bad: 'for() { new StringBuilder() }', good: '复用对象' },
    { name: '无界队列', bad: 'Executors.newFixedThreadPool', good: '有界队列 + 拒绝策略' },
    { name: '缓存穿透', bad: 'if (cache==null) db.query()', good: '加锁防穿透' },
    { name: 'N+1 查询', bad: 'for(u:users) dao.get(u.id)', good: '批量查询' }
];
