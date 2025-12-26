//! AST Engine - Tree-sitter Java 分析
//! 
//! 🛰️ 雷达扫描：检测性能反模式

use serde_json::{json, Value};
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;

/// 问题严重级别
#[derive(Debug, Clone, Copy)]
pub enum Severity {
    P0, // 严重
    P1, // 警告
}

/// AST 检测问题
#[derive(Debug)]
pub struct AstIssue {
    pub severity: Severity,
    pub issue_type: String,
    pub file: String,
    pub line: usize,
    pub description: String,
}

/// 全项目雷达扫描
pub fn radar_scan(code_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(code_path);
    let mut issues: Vec<AstIssue> = Vec::new();
    let mut file_count = 0;
    
    // 遍历所有 Java 文件
    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if file_path.extension().map_or(false, |ext| ext == "java") {
            file_count += 1;
            
            // 读取文件内容
            if let Ok(content) = std::fs::read_to_string(file_path) {
                let file_name = file_path.to_string_lossy().to_string();
                let file_issues = analyze_java_code(&content, &file_name);
                issues.extend(file_issues);
            }
        }
    }
    
    // 生成报告
    let p0_count = issues.iter().filter(|i| matches!(i.severity, Severity::P0)).count();
    let p1_count = issues.iter().filter(|i| matches!(i.severity, Severity::P1)).count();
    
    let mut report = format!(
        "## 🛰️ 雷达扫描结果\n\n\
        **扫描**: {} 个 Java 文件\n\
        **发现**: {} 个嫌疑点 (P0: {}, P1: {})\n\n",
        file_count, issues.len(), p0_count, p1_count
    );
    
    if p0_count > 0 {
        report.push_str("### 🔴 P0 严重嫌疑\n\n");
        for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P0)) {
            report.push_str(&format!(
                "- **{}** - `{}:{}` - {}\n",
                issue.issue_type, issue.file, issue.line, issue.description
            ));
        }
        report.push('\n');
    }
    
    if p1_count > 0 {
        report.push_str("### 🟡 P1 警告\n\n");
        for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P1)).take(10) {
            report.push_str(&format!(
                "- **{}** - `{}:{}` - {}\n",
                issue.issue_type, issue.file, issue.line, issue.description
            ));
        }
    }
    
    Ok(json!(report))
}

/// 单文件扫描
pub fn scan_source_code(code: &str, file_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let issues = analyze_java_code(code, file_path);
    
    let mut report = format!("## 🛰️ 扫描: {}\n\n", file_path);
    
    if issues.is_empty() {
        report.push_str("✅ 未发现明显性能问题\n");
    } else {
        for issue in &issues {
            let emoji = match issue.severity {
                Severity::P0 => "🔴",
                Severity::P1 => "🟡",
            };
            report.push_str(&format!(
                "{} **{}** (行 {}) - {}\n",
                emoji, issue.issue_type, issue.line, issue.description
            ));
        }
    }
    
    Ok(json!(report))
}

/// 分析 Java 代码（基于正则模式匹配）
fn analyze_java_code(code: &str, file_path: &str) -> Vec<AstIssue> {
    let mut issues = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    let file_name = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    
    // 检测模式 - 来自 checklist-data.ts
    let patterns: Vec<(&str, &str, Severity, &str)> = vec![
        // P0 - 代码级放大效应
        ("N_PLUS_ONE", "循环内 IO/计算", Severity::P0, 
         r"(?i)for\s*\([^)]+\)\s*\{[^}]*(dao|repository|mapper|jdbc|select|insert|update|delete|http|client)[^}]*\}"),
        ("NESTED_LOOP", "嵌套循环 O(N*M)", Severity::P0,
         r"for\s*\([^)]+\)\s*\{[^}]*for\s*\([^)]+\)"),
        ("OBJECT_IN_LOOP", "循环内创建对象", Severity::P1,
         r"for\s*\([^)]+\)\s*\{[^}]*new\s+\w+\s*\("),
        
        // P0 - 锁与并发
        ("SYNC_METHOD", "synchronized 方法级锁", Severity::P0,
         r"synchronized\s+(public|private|protected)"),
        ("SYNC_BLOCK_LARGE", "synchronized 大代码块", Severity::P1,
         r"synchronized\s*\([^)]+\)\s*\{"),
        ("ATOMIC_SPIN", "Atomic 自旋", Severity::P1,
         r"(AtomicInteger|AtomicLong|AtomicReference)\s*[<\s]"),
        
        // P0 - 资源管理
        ("THREADLOCAL_LEAK", "ThreadLocal 未 remove", Severity::P0,
         r"ThreadLocal\s*<"),
        ("UNBOUNDED_POOL", "无界线程池", Severity::P0,
         r"Executors\s*\.\s*(newCachedThreadPool|newScheduledThreadPool|newSingleThreadExecutor)"),
        ("UNBOUNDED_CACHE", "无界缓存 static Map", Severity::P0,
         r"static\s+.*Map\s*<[^>]+>\s*\w+\s*=\s*new"),
        
        // P1 - IO/阻塞
        ("NO_TIMEOUT", "可能无超时", Severity::P1,
         r"(HttpClient|RestTemplate|OkHttp|WebClient)\s*[.\s]"),
        ("BLOCKING_IO", "同步文件 IO", Severity::P1,
         r"new\s+File(Input|Output)Stream"),
        
        // P2 - 异常处理
        ("EXCEPTION_SWALLOW", "异常被吞没", Severity::P1,
         r"catch\s*\([^)]+\)\s*\{\s*(//|/\*|\}|e\.print)"),
        ("EXCEPTION_IGNORE", "空 catch 块", Severity::P0,
         r"catch\s*\([^)]+\)\s*\{\s*\}"),
        
        // P1 - 字符串
        ("STRING_CONCAT_LOOP", "循环内字符串拼接", Severity::P1,
         r"for\s*\([^)]+\)\s*\{.*\+=.*String"),
    ];
    
    // 编译正则并匹配
    for (issue_type, desc, severity, pattern) in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(code) {
                if let Some(mat) = regex.find(code) {
                    let line_num = code[..mat.start()].matches('\n').count() + 1;
                    
                    // 去重
                    let exists = issues.iter().any(|i: &AstIssue| 
                        i.issue_type == *issue_type && i.line == line_num
                    );
                    
                    if !exists {
                        issues.push(AstIssue {
                            severity: *severity,
                            issue_type: issue_type.to_string(),
                            file: file_name.clone(),
                            line: line_num,
                            description: desc.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    issues
}
