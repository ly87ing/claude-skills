/**
 * Audit 模块 - 代码审计
 * 
 * 核心能力：
 * 1. 自适应规则加载：根据症状 tags 筛选规则
 * 2. 精准狙击：有坐标时优先扫描嫌疑文件
 * 3. 证据链匹配：日志行号 ↔ 代码行号 ±5 行匹配
 */

import * as fs from 'fs';
import * as path from 'path';
import { AuditRule, CrimeScene, AuditFinding, Symptom, Severity } from '../types.js';

// ========== 审计规则库 ==========

export const AUDIT_RULES: AuditRule[] = [
    // ===== P0 放大效应 =====
    {
        id: 'loop-io',
        severity: 'P0',
        tags: ['cpu', 'slow'],
        name: '循环内 IO (N+1 放大)',
        pattern: '(for|while|forEach)\\s*\\([^)]*\\)\\s*\\{[\\s\\S]{0,500}\\.(dao|mapper|repository|client|http|rpc)\\.',
        message: '循环内调用 DAO/RPC，100 次循环 = 100 次网络往返',
        fix: '批量查询替代循环查询'
    },
    {
        id: 'nested-loop',
        severity: 'P0',
        tags: ['cpu', 'slow'],
        name: '嵌套循环 (笛卡尔积)',
        pattern: 'for\\s*\\([^)]*\\)\\s*\\{[\\s\\S]{0,300}for\\s*\\([^)]*\\)',
        message: 'O(N*M) 复杂度，100x100=1万次',
        fix: '使用 Map 降到 O(N+M)'
    },

    // ===== P0 内存泄露 =====
    {
        id: 'threadlocal-leak',
        severity: 'P0',
        tags: ['memory'],
        name: 'ThreadLocal 泄露',
        pattern: 'ThreadLocal',
        message: 'ThreadLocal 必须在 finally 中 remove',
        fix: 'try { ... } finally { threadLocal.remove(); }'
    },
    {
        id: 'static-map',
        severity: 'P0',
        tags: ['memory'],
        name: '无界静态缓存',
        pattern: 'static\\s+(?:final\\s+)?(?:Map|HashMap|ConcurrentHashMap)',
        message: 'static Map 只增不删会导致 OOM',
        fix: '使用 Caffeine/Guava Cache 带 TTL 和 Size 限制'
    },

    // ===== P0 锁竞争 =====
    {
        id: 'synchronized-method',
        severity: 'P0',
        tags: ['cpu', 'slow'],
        name: '方法级同步锁',
        pattern: 'synchronized\\s+\\w+\\s+\\w+\\s*\\([^)]*\\)\\s*\\{',
        message: '方法级锁粒度过大，并发变串行',
        fix: '细化锁粒度，只锁关键代码块'
    },
    {
        id: 'lock-io',
        severity: 'P0',
        tags: ['cpu', 'slow'],
        name: '锁内 IO',
        pattern: 'synchronized\\s*\\([^)]*\\)\\s*\\{[\\s\\S]{0,500}\\.(http|rpc|dao|client)',
        message: '锁内进行 IO 操作，严重阻塞',
        fix: '锁外获取数据，锁内只做计算'
    },

    // ===== P0 资源泄露 =====
    {
        id: 'unclosed-stream',
        severity: 'P0',
        tags: ['resource'],
        name: '资源未关闭',
        pattern: 'new\\s+(FileInputStream|FileOutputStream|BufferedReader|Connection)',
        message: '资源可能未正确关闭',
        fix: '使用 try-with-resources'
    },
    {
        id: 'cached-threadpool',
        severity: 'P0',
        tags: ['resource', 'memory'],
        name: '无界线程池',
        pattern: 'Executors\\.newCachedThreadPool',
        message: '无界线程池会无限创建线程导致 OOM',
        fix: '使用 ThreadPoolExecutor 有界线程池'
    },

    // ===== P1 性能问题 =====
    {
        id: 'system-out',
        severity: 'P1',
        tags: ['slow'],
        name: 'System.out 同步锁',
        pattern: 'System\\.out\\.print',
        message: 'System.out 有同步锁，生产禁用',
        fix: '使用 SLF4J 等日志框架'
    },
    {
        id: 'regex-compile',
        severity: 'P1',
        tags: ['cpu'],
        name: '正则反复编译',
        pattern: 'Pattern\\.compile\\s*\\([^)]*\\)',
        message: '如在循环中，应预编译为静态常量',
        fix: 'private static final Pattern PATTERN = Pattern.compile(...)'
    },
    {
        id: 'string-concat-loop',
        severity: 'P1',
        tags: ['memory', 'gc'],
        name: '循环内字符串拼接',
        pattern: '(for|while)\\s*\\([^)]*\\)\\s*\\{[\\s\\S]{0,200}\\+\\s*=\\s*["\']',
        message: '循环内 += 字符串创建大量临时对象',
        fix: '使用 StringBuilder'
    },

    // ===== P1 超时配置 =====
    {
        id: 'no-timeout',
        severity: 'P1',
        tags: ['slow', 'resource'],
        name: '无超时设置',
        pattern: '(HttpClient|RestTemplate|OkHttp)(?![\\s\\S]{0,100}timeout)',
        message: 'HTTP 客户端未配置超时',
        fix: '配置 connectTimeout 和 readTimeout'
    }
];

// ========== 递归扫描目录 ==========

function walkDir(dir: string, callback: (file: string) => void, depth: number = 0) {
    if (depth > 10) return;  // 防止过深递归

    try {
        const files = fs.readdirSync(dir);
        for (const file of files) {
            const fullPath = path.join(dir, file);
            const stat = fs.statSync(fullPath);

            if (stat.isDirectory()) {
                // 跳过常见无关目录
                if (['node_modules', 'target', 'build', '.git', '.idea'].includes(file)) {
                    continue;
                }
                walkDir(fullPath, callback, depth + 1);
            } else if (file.endsWith('.java')) {
                callback(fullPath);
            }
        }
    } catch (err) {
        // 忽略权限错误等
    }
}

// ========== 智能审计 ==========

/**
 * 执行智能审计
 * 
 * @param codeRoot 代码根目录
 * @param crimeScenes 日志中提取的代码坐标（嫌疑人）
 * @param symptoms 用户描述的症状
 */
export function runSmartAudit(
    codeRoot: string,
    crimeScenes: CrimeScene[] = [],
    symptoms: Symptom[] = []
): AuditFinding[] {
    const findings: AuditFinding[] = [];
    const suspectFiles = new Set(crimeScenes.map(c => c.file));

    walkDir(codeRoot, (filePath) => {
        const fileName = path.basename(filePath);
        let content: string;

        try {
            content = fs.readFileSync(filePath, 'utf-8');
        } catch {
            return;
        }

        // 判断是否是嫌疑文件
        const isSuspect = suspectFiles.has(fileName);
        const suspectInfo = crimeScenes.find(c => c.file === fileName);

        // 动态筛选规则
        const activeRules = AUDIT_RULES.filter(rule => {
            // 嫌疑文件：跑所有 P0/P1 规则
            if (isSuspect) return rule.severity === 'P0' || rule.severity === 'P1';
            // 有症状：跑 P0 + 匹配症状的规则
            if (symptoms.length > 0) {
                if (rule.severity === 'P0') return true;
                if (rule.tags?.some(t => symptoms.includes(t))) return true;
                return false;
            }
            // 默认：只跑 P0
            return rule.severity === 'P0';
        });

        // 执行规则匹配
        for (const rule of activeRules) {
            try {
                const regex = new RegExp(rule.pattern, 'g');
                let match;

                while ((match = regex.exec(content)) !== null) {
                    // 计算行号
                    const lineNum = content.substring(0, match.index).split('\n').length;

                    // 证据链匹配：日志行号 ↔ 代码行号 ±5 行
                    let correlation: string | undefined;
                    let findingType: 'ROOT_CAUSE' | 'RISK' = 'RISK';

                    if (suspectInfo && Math.abs(suspectInfo.line - lineNum) <= 5) {
                        correlation = `🎯 与堆栈 ${suspectInfo.file}:${suspectInfo.line} 匹配 (±${Math.abs(suspectInfo.line - lineNum)} 行)`;
                        findingType = 'ROOT_CAUSE';
                    }

                    findings.push({
                        type: findingType,
                        ruleId: rule.id,
                        ruleName: rule.name,
                        severity: rule.severity,
                        file: path.relative(codeRoot, filePath),
                        line: lineNum,
                        evidence: match[0].substring(0, 100),
                        note: rule.message,
                        correlation
                    });
                }
            } catch {
                // 正则错误忽略
            }
        }
    });

    // 排序：ROOT_CAUSE 优先，然后按严重级别
    findings.sort((a, b) => {
        if (a.type !== b.type) return a.type === 'ROOT_CAUSE' ? -1 : 1;
        if (a.severity !== b.severity) return a.severity < b.severity ? -1 : 1;
        return 0;
    });

    return findings;
}
