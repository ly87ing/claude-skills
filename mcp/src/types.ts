/**
 * Omni-Engine 类型定义
 */

// 严重级别
export type Severity = 'P0' | 'P1' | 'P2';

// 症状类型
export type Symptom = 'memory' | 'cpu' | 'slow' | 'resource' | 'backlog' | 'gc';

// 审计规则
export interface AuditRule {
    id: string;
    severity: Severity;
    name: string;
    pattern: string;      // 正则表达式
    message: string;      // 问题描述
    fix?: string;         // 修复建议
    tags?: Symptom[];     // 关联症状
    tech?: string;        // 适用技术栈
}

// 案发现场（日志中提取的代码坐标）
export interface CrimeScene {
    file: string;         // 文件名
    line: number;         // 行号
    reason: string;       // 来源说明
}

// 日志异常特征
export interface LogAnomaly {
    pattern: string;      // 归一化后的日志模式
    count: number;        // 出现次数
    rate: number;         // 每秒频率
    duration: number;     // 持续时间（秒）
    example: string;      // 原始示例
}

// 日志分析结果
export interface LogAnalysisResult {
    summary: string;      // 摘要（给 LLM 看的精简版）
    anomalies: LogAnomaly[];  // 异常高频日志
    errors: string[];     // 错误列表
    coordinates: CrimeScene[];  // 提取的代码坐标
}

// 审计发现
export interface AuditFinding {
    type: 'ROOT_CAUSE' | 'RISK';  // 根因 vs 风险
    ruleId: string;
    ruleName: string;
    severity: Severity;
    file: string;
    line: number;
    evidence: string;     // 匹配的代码片段
    note: string;         // 说明
    correlation?: string; // 证据链关联说明
}

// 诊断报告
export interface InvestigationReport {
    status: 'Success' | 'Error';
    mode: 'Evidence-Driven' | 'Symptom-Driven' | 'Baseline-Check';
    rootCauses: AuditFinding[];
    otherRisks: AuditFinding[];
    logAnalysis?: string[];
    images?: any[];
}
