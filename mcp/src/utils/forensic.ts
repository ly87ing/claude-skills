/**
 * Forensic 模块 - 日志时序分析 + 坐标提取
 * 
 * 核心能力：
 * 1. 时序折叠算法：将高频重复日志压缩为统计信息
 * 2. 坐标提取：从堆栈中提取 (File.java:123) 格式的代码位置
 * 3. 错误摘要：提取 Exception/ERROR 信息
 */

import * as fs from 'fs';
import * as path from 'path';
import { CrimeScene, LogAnomaly, LogAnalysisResult } from '../types.js';

// ========== 日志归一化 ==========

/**
 * 归一化日志行（去除时间戳、数字、UUID 等变量部分）
 * 目的：识别重复模式
 */
function normalizeLogLine(line: string): string {
    return line
        // 去除常见时间戳格式
        .replace(/\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}[.,]?\d*/g, '{TIME}')
        // 去除纯数字
        .replace(/\b\d+\b/g, '{N}')
        // 去除 UUID
        .replace(/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/gi, '{UUID}')
        // 去除 IP 地址
        .replace(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/g, '{IP}')
        // 截断过长内容
        .trim()
        .substring(0, 150);
}

/**
 * 从日志行提取时间戳（毫秒）
 */
function extractTimestamp(line: string): number | null {
    // 匹配常见格式：2024-01-01 12:00:00 或 2024-01-01T12:00:00
    const patterns = [
        /(\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2})/,
        /(\d{2}:\d{2}:\d{2}[.,]\d{3})/  // HH:mm:ss.SSS
    ];

    for (const pattern of patterns) {
        const match = line.match(pattern);
        if (match) {
            const ts = Date.parse(match[1].replace(' ', 'T'));
            if (!isNaN(ts)) return ts;
        }
    }
    return null;
}

// ========== 坐标提取 ==========

/**
 * 从日志内容中提取代码坐标（堆栈信息）
 * 匹配格式：(OrderService.java:45) 或 at com.xxx.OrderService.method(OrderService.java:45)
 */
function extractCoordinates(content: string): CrimeScene[] {
    const scenes: CrimeScene[] = [];
    const seen = new Set<string>();

    // 匹配 Java 堆栈格式
    const regex = /\((\w+\.java):(\d+)\)/g;
    let match;

    while ((match = regex.exec(content)) !== null) {
        const key = `${match[1]}:${match[2]}`;
        if (!seen.has(key)) {
            seen.add(key);
            scenes.push({
                file: match[1],
                line: parseInt(match[2]),
                reason: 'Stack Trace'
            });
        }
    }

    // 按出现频率排序（频繁出现的可能是热点）
    return scenes.slice(0, 20);  // 最多返回 20 个坐标
}

// ========== 时序折叠分析 ==========

/**
 * 分析日志文件，返回精简摘要
 * 
 * @param filePath 日志文件路径
 * @param maxLines 最大读取行数（防止内存溢出）
 */
export function analyzeLog(filePath: string, maxLines: number = 50000): LogAnalysisResult {
    let content: string;

    try {
        // 读取文件（生产环境应使用 Stream）
        const stat = fs.statSync(filePath);
        if (stat.size > 100 * 1024 * 1024) {
            // 文件超过 100MB，只读取头尾
            const fd = fs.openSync(filePath, 'r');
            const headBuffer = Buffer.alloc(5 * 1024 * 1024);
            const tailBuffer = Buffer.alloc(5 * 1024 * 1024);
            fs.readSync(fd, headBuffer, 0, headBuffer.length, 0);
            fs.readSync(fd, tailBuffer, 0, tailBuffer.length, stat.size - tailBuffer.length);
            fs.closeSync(fd);
            content = headBuffer.toString('utf-8') + '\n...[TRUNCATED]...\n' + tailBuffer.toString('utf-8');
        } else {
            content = fs.readFileSync(filePath, 'utf-8');
        }
    } catch (err) {
        return {
            summary: `Error reading log file: ${err}`,
            anomalies: [],
            errors: [],
            coordinates: []
        };
    }

    const lines = content.split('\n').slice(0, maxLines);
    const coordinates = extractCoordinates(content);

    // ===== 时序折叠分析 =====
    const patternMap = new Map<string, {
        count: number;
        firstTs: number | null;
        lastTs: number | null;
        example: string;
    }>();

    for (const line of lines) {
        if (!line.trim()) continue;

        const normalized = normalizeLogLine(line);
        const ts = extractTimestamp(line);

        if (!patternMap.has(normalized)) {
            patternMap.set(normalized, {
                count: 0,
                firstTs: ts,
                lastTs: ts,
                example: line.substring(0, 200)
            });
        }

        const entry = patternMap.get(normalized)!;
        entry.count++;
        if (ts) entry.lastTs = ts;
    }

    // 计算频率并筛选异常
    const anomalies: LogAnomaly[] = [];

    for (const [pattern, data] of patternMap) {
        const duration = (data.lastTs && data.firstTs)
            ? (data.lastTs - data.firstTs) / 1000
            : 0;
        const rate = duration > 0 ? data.count / duration : 0;

        // 筛选条件：次数 > 1000 或 频率 > 10/s
        if (data.count > 1000 || rate > 10) {
            anomalies.push({
                pattern,
                count: data.count,
                rate: Math.round(rate * 10) / 10,
                duration: Math.round(duration),
                example: data.example
            });
        }
    }

    // 按频率排序
    anomalies.sort((a, b) => b.rate - a.rate);

    // ===== 错误提取 =====
    const errors = lines
        .filter(line => /Exception|ERROR|FATAL|Caused by/i.test(line))
        .slice(0, 30);  // 最多 30 条错误

    // ===== 生成摘要 =====
    let summary = `### 日志分析: ${path.basename(filePath)}\n\n`;

    if (anomalies.length > 0) {
        summary += `🚨 **高频日志异常 (疑似死循环/风暴):**\n`;
        anomalies.slice(0, 5).forEach((a, i) => {
            summary += `${i + 1}. [${a.rate}/s, ${a.count}次] ${a.example.substring(0, 80)}...\n`;
        });
        summary += '\n';
    }

    if (errors.length > 0) {
        summary += `❌ **错误日志 (Top ${Math.min(errors.length, 10)}):**\n`;
        errors.slice(0, 10).forEach((e, i) => {
            summary += `${i + 1}. ${e.substring(0, 100)}...\n`;
        });
        summary += '\n';
    }

    if (coordinates.length > 0) {
        summary += `📍 **代码坐标 (来自堆栈):**\n`;
        coordinates.slice(0, 5).forEach(c => {
            summary += `- ${c.file}:${c.line}\n`;
        });
    }

    return {
        summary,
        anomalies: anomalies.slice(0, 10),
        errors,
        coordinates
    };
}

/**
 * 读取图片为 Base64
 */
export function readImageAsBase64(filePath: string): string | null {
    try {
        const buffer = fs.readFileSync(filePath);
        return buffer.toString('base64');
    } catch {
        return null;
    }
}

/**
 * 扫描目录中的日志和图片
 */
export function scanEvidenceDir(dirPath: string): {
    logs: LogAnalysisResult[];
    images: Array<{ path: string; base64: string; mimeType: string }>;
} {
    const result = {
        logs: [] as LogAnalysisResult[],
        images: [] as Array<{ path: string; base64: string; mimeType: string }>
    };

    if (!fs.existsSync(dirPath)) {
        return result;
    }

    const files = fs.readdirSync(dirPath);

    for (const file of files) {
        const fullPath = path.join(dirPath, file);
        const stat = fs.statSync(fullPath);

        if (!stat.isFile()) continue;

        // 日志文件
        if (/\.(log|txt|out)$/i.test(file)) {
            result.logs.push(analyzeLog(fullPath));
        }
        // 图片文件
        else if (/\.(png|jpg|jpeg|gif)$/i.test(file)) {
            const base64 = readImageAsBase64(fullPath);
            if (base64) {
                const mimeType = file.endsWith('.png') ? 'image/png' : 'image/jpeg';
                result.images.push({ path: fullPath, base64, mimeType });
            }
        }
    }

    return result;
}
