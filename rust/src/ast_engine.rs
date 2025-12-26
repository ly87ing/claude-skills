//! AST Engine - 双遍语义分析引擎
//!
//! 🛰️ 雷达扫描：检测性能反模式
//!
//! v9.0 架构重构:
//! - AST 规则优先 (tree_sitter_java.rs)
//! - Regex 仅用于无法用 AST 表达的规则 (SQL 检测、HTTP 客户端提示)
//! - 统一规则 ID，消除重复检测
//!
//! 优化点：
//! 1. 使用 once_cell 静态编译正则，避免重复创建
//! 2. 过滤注释内容，避免误报
//! 3. 集成 Tree-sitter AST 分析 (v5.0)
//! 4. 并行文件扫描 (rayon) (v5.1)
//! 5. Dockerfile 扫描 (v5.1)
//! 6. 双遍语义引擎 (v8.0)
//! 7. 规则去重，消除 Regex/AST 冲突 (v9.0)

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;
use walkdir::WalkDir;
use rayon::prelude::*;

use crate::scanner::{CodeAnalyzer, Issue as ScannerIssue, Severity as ScannerSeverity};
use crate::scanner::tree_sitter_java::JavaTreeSitterAnalyzer;
use crate::scanner::config::LineBasedConfigAnalyzer;
use crate::scanner::dockerfile::DockerfileAnalyzer;

// ============================================================================
// 静态编译正则表达式（只编译一次，全局复用）
// ============================================================================
//
// v9.0 说明：大部分规则已迁移至 tree_sitter_java.rs 使用 AST 分析
// 以下只保留「无法用 AST 表达」或「Regex 更高效」的规则：
// 1. SQL 字符串检测 (需要匹配字符串字面量内容)
// 2. HTTP 客户端使用提示 (仅作为线索，非精确检测)
// 3. 无界缓存 Map/List (static 字段的泛型类型匹配)
// 4. 异常处理 (仅打印/吞没，作为 AST 规则的补充)
// ============================================================================

/// 注释匹配正则（用于过滤）
static COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"//.*$|/\*[\s\S]*?\*/").unwrap()
});

// === 数据库 SQL 检测 (无法用 AST 精确匹配字符串内容) ===
static RE_SELECT_STAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"["']SELECT\s+\*\s+FROM"#).unwrap()
});
static RE_LIKE_LEADING_WILDCARD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"LIKE\s+['"]%"#).unwrap()
});

// === HTTP 客户端提示 (仅作为线索提示检查超时配置) ===
static RE_HTTP_CLIENT_USAGE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(HttpClient|RestTemplate|OkHttp|WebClient)\s*\.").unwrap()
});

// === 无界缓存检测 (static 泛型字段，AST 规则作为主要检测) ===
// 注意: STATIC_COLLECTION_AST 已在 tree_sitter_java.rs 中实现
// 这里保留作为补充，用于检测更复杂的泛型声明模式
static RE_UNBOUNDED_CACHE_MAP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"static\s+.*Map\s*<[^>]+>\s*\w+\s*=\s*new").unwrap()
});
static RE_UNBOUNDED_CACHE_LIST: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"static\s+.*(List|Set)\s*<[^>]+>\s*\w+\s*=\s*new").unwrap()
});

// === 异常处理补充检测 (AST 主检测，这里作为补充) ===
static RE_EXCEPTION_SWALLOW: Lazy<Regex> = Lazy::new(|| {
    // catch 后仅打印 (e.printStackTrace 等)
    Regex::new(r"catch\s*\([^)]+\)\s*\{[^}]*\.print").unwrap()
});

// === 缓存配置检测 (需要额外上下文验证) ===
static RE_CACHE_NO_EXPIRE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(Caffeine|CacheBuilder)\s*\.\s*newBuilder").unwrap()
});

// ============================================================================
// 规则定义
// ============================================================================

/// 问题严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// 规则配置
struct Rule {
    id: &'static str,
    description: &'static str,
    severity: Severity,
    regex: &'static Lazy<Regex>,
}

/// 精简规则集 (v9.0)
///
/// 只保留「无法用 AST 表达」或「作为 AST 规则补充」的 Regex 规则：
/// - SQL 检测：需要匹配字符串字面量内容
/// - HTTP 客户端提示：仅作为线索
/// - 无界缓存：补充 AST 的泛型检测
/// - 异常处理：补充 AST 的空 catch 检测
fn get_rules() -> Vec<Rule> {
    vec![
        // === SQL 检测 (无法用 AST 精确匹配字符串内容) ===
        Rule { id: "SELECT_STAR", description: "SELECT * 查询，建议明确指定字段", severity: Severity::P1, regex: &RE_SELECT_STAR },
        Rule { id: "LIKE_LEADING_WILDCARD", description: "LIKE '%xxx' 前导通配符导致全表扫描", severity: Severity::P0, regex: &RE_LIKE_LEADING_WILDCARD },

        // === HTTP 客户端提示 (仅作为线索) ===
        Rule { id: "HTTP_CLIENT_CHECK_TIMEOUT", description: "HTTP 客户端使用，请确认已配置超时", severity: Severity::P1, regex: &RE_HTTP_CLIENT_USAGE },

        // === 无界缓存补充检测 ===
        // 主检测由 STATIC_COLLECTION_AST 完成，这里检测更复杂的泛型模式
        Rule { id: "UNBOUNDED_CACHE_MAP", description: "无界缓存 static Map (请配置大小限制)", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_MAP },
        Rule { id: "UNBOUNDED_CACHE_LIST", description: "无界缓存 static List/Set (请配置大小限制)", severity: Severity::P0, regex: &RE_UNBOUNDED_CACHE_LIST },

        // === 异常处理补充检测 ===
        // 主检测由 EMPTY_CATCH_AST 完成，这里检测仅打印的情况
        Rule { id: "EXCEPTION_SWALLOW", description: "异常被吞没 (仅打印)，建议正确处理或重抛", severity: Severity::P1, regex: &RE_EXCEPTION_SWALLOW },

        // === 缓存配置检测 (需要额外上下文验证) ===
        // 注意：这只是提示，实际需要检查是否配置了 expire/maximumSize
        // Rule { id: "CACHE_NO_EXPIRE", ... } -- 移动到 analyze_java_code 中做特殊处理
    ]
}

// Helper to convert ScannerIssue to AstIssue
fn convert_issue(issue: ScannerIssue) -> AstIssue {
    let sev = match issue.severity {
        ScannerSeverity::P0 => Severity::P0,
        ScannerSeverity::P1 => Severity::P1,
    };
    AstIssue {
        severity: sev,
        issue_type: issue.id,
        file: issue.file,
        line: issue.line,
        description: issue.description,
    }
}

// ============================================================================
// 核心扫描函数
// ============================================================================

/// 全项目雷达扫描 (v8.0 双遍架构)
/// 
/// compact: true 时只返回 P0，每个 issue 只有 id/file/line
/// max_p1: compact=false 时最多返回的 P1 数量
pub fn radar_scan(code_path: &str, compact: bool, max_p1: usize) -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(code_path);
    let is_dir = path.is_dir();
    
    // 收集所有待扫描文件
    let entries: Vec<_> = WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    let file_count = entries.len();

    // 初始化分析器 (Arc 共享，只编译一次 queries)
    let java_analyzer = std::sync::Arc::new(JavaTreeSitterAnalyzer::new()?);
    let config_analyzer = LineBasedConfigAnalyzer::new().ok();
    let docker_analyzer = DockerfileAnalyzer::new().ok();

    // === Phase 1: Indexing (构建全局符号表) ===
    let mut symbol_table = crate::symbol_table::SymbolTable::new();
    
    // 只有目录扫描且包含 Java 文件时才进行索引构建
    if is_dir {
        // 使用并行迭代器进行索引
        // 注意：由于 SymbolTable 需要合并，我们使用 map/reduce
        let java_files: Vec<_> = entries.iter()
            .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some("java"))
            .collect();
            
        if !java_files.is_empty() {
            // Log indexing (optional)
            // println!("Phase 1: Indexing {} Java files...", java_files.len());
            
            let tables: Vec<crate::symbol_table::SymbolTable> = java_files.par_iter().map(|entry| {
                let mut local_table = crate::symbol_table::SymbolTable::new();
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok((Some(type_info), bindings)) = java_analyzer.extract_symbols(&content, entry.path()) {
                        // 注册类和字段
                        let class_name = type_info.name.clone();
                        local_table.register_class(type_info);
                        for binding in bindings {
                            local_table.register_field(&class_name, binding);
                        }
                    }
                }
                local_table
            }).collect();
            
            // Merge all tables
            for table in tables {
                for (name, info) in table.classes {
                    symbol_table.classes.insert(name, info);
                }
                for (key, binding) in table.fields {
                    symbol_table.fields.insert(key, binding);
                }
                for (key, info) in table.methods {
                    symbol_table.methods.insert(key, info);
                }
            }
        }
    }
    
    let symbol_table_ref = &symbol_table;

    // === Phase 2: Deep Analysis (深度扫描) ===
    // 使用 Mutex 保护共享状态 (rayon 并行安全)
    let issues: Mutex<Vec<AstIssue>> = Mutex::new(Vec::new());

    // 并行处理文件
    entries.par_iter().for_each(|entry| {
        let file_path = entry.path();
        let file_name_str = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 本线程的 issues
        let mut local_issues: Vec<AstIssue> = Vec::new();

        if ext == "java" {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 1. Regex Analysis (Legacy - still useful for some non-AST rules)
                let legacy = analyze_java_code(&content, &file_path.to_string_lossy());
                local_issues.extend(legacy);

                // 2. AST Analysis (with Context)
                // 传入全局 SymbolTable 引用
                let ctx = if is_dir { Some(symbol_table_ref) } else { None };
                
                if let Ok(ast_results) = java_analyzer.analyze_with_context(&content, file_path, ctx) {
                    local_issues.extend(ast_results.into_iter().map(convert_issue));
                }
            }
        } else if ["yml", "yaml", "properties"].contains(&ext) {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 3. Config Analysis
                if let Some(analyzer) = &config_analyzer {
                    if let Ok(config_results) = analyzer.analyze(&content, file_path) {
                        local_issues.extend(config_results.into_iter().map(convert_issue));
                    }
                }
            }
        } else if file_name_str == "Dockerfile" || file_name_str.starts_with("Dockerfile.") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // 4. Dockerfile Analysis (v5.1 NEW)
                if let Some(analyzer) = &docker_analyzer {
                    if let Ok(docker_results) = analyzer.analyze(&content, file_path) {
                        local_issues.extend(docker_results.into_iter().map(convert_issue));
                    }
                }
            }
        }

        // 合并到全局 issues
        if !local_issues.is_empty() {
            // 使用 unwrap_or_else 处理 poisoned mutex（如果持锁线程 panic）
            let mut global = issues.lock().unwrap_or_else(|e| e.into_inner());
            global.extend(local_issues);
        }
    });

    // 安全地解包：如果 mutex 被 poisoned，仍然获取内部数据
    let issues = issues.into_inner().unwrap_or_else(|e| e.into_inner());
    let p0_count = issues.iter().filter(|i| matches!(i.severity, Severity::P0)).count();
    let p1_count = issues.iter().filter(|i| matches!(i.severity, Severity::P1)).count();

    // === 根据 compact 模式生成不同报告 ===
    if compact {
        // 紧凑模式：只返回 P0，精简格式
        let mut report = format!(
            "## 🛰️ 雷达扫描 (v8.0 双遍引擎)\n\n**P0**: {p0_count} | **P1**: {p1_count} | **文件**: {file_count}\n\n"
        );

        if p0_count > 0 {
            for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P0)) {
                report.push_str(&format!(
                    "- `{}` {}:{}\n",
                    issue.issue_type, issue.file, issue.line
                ));
            }
        } else {
            report.push_str("✅ 无 P0 问题\n");
        }

        if p1_count > 0 {
            report.push_str(&format!("\n*（{p1_count} 个 P1 警告已省略，使用 compact=false 查看）*\n"));
        }

        Ok(json!(report))
    } else {
        // 完整模式
        let mut report = format!(
            "## 🛰️ 雷达扫描结果 (v8.0 双遍引擎)\n\n\
            **扫描**: {} 个文件\n\
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
            report.push_str(&format!("### 🟡 P1 警告 (显示前 {max_p1})\n\n"));
            for issue in issues.iter().filter(|i| matches!(i.severity, Severity::P1)).take(max_p1) {
                report.push_str(&format!(
                    "- **{}** - `{}:{}` - {}\n",
                    issue.issue_type, issue.file, issue.line, issue.description
                ));
            }
        }

        Ok(json!(report))
    }
}

/// 单文件扫描
pub fn scan_source_code(code: &str, file_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut issues = Vec::new();
    let path = Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext == "java" {
        // Regex
        issues.extend(analyze_java_code(code, file_path));
        // AST
        if let Ok(analyzer) = JavaTreeSitterAnalyzer::new() {
             if let Ok(res) = analyzer.analyze(code, path) {
                 issues.extend(res.into_iter().map(convert_issue));
             }
        }
    } else if ["yml", "yaml", "properties"].contains(&ext) {
        // Config
        if let Ok(analyzer) = LineBasedConfigAnalyzer::new() {
             if let Ok(res) = analyzer.analyze(code, path) {
                 issues.extend(res.into_iter().map(convert_issue));
             }
        }
    }

    let mut report = format!("## 🛰️ 扫描: {file_path}\n\n");

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

/// 分析 Java 代码（高性能版本 - Legacy Regex）
fn analyze_java_code(code: &str, file_path: &str) -> Vec<AstIssue> {
    let mut issues = Vec::new();
    let file_name = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    // 1. 移除注释，避免误报
    let code_without_comments = COMMENT_REGEX.replace_all(code, "");

    // 2. 特殊检测：ThreadLocal (MIGRATED TO AST -> DISABLED HERE)
    /*
    if RE_THREADLOCAL.is_match(&code_without_comments) {
        if !code_without_comments.contains(".remove()") {
            if let Some(mat) = RE_THREADLOCAL.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;
                issues.push(AstIssue {
                    severity: Severity::P0,
                    issue_type: "THREADLOCAL_LEAK".to_string(),
                    file: file_name.clone(),
                    line: line_num,
                    description: "ThreadLocal 未调用 remove()，线程池复用会导致内存泄露".to_string(),
                });
            }
        }
    }
    */

    // 3. 特殊检测：Cache 需要 expire 配置
    if RE_CACHE_NO_EXPIRE.is_match(&code_without_comments)
        && !code_without_comments.contains("expire") && !code_without_comments.contains("maximumSize") {
            if let Some(mat) = RE_CACHE_NO_EXPIRE.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;
                issues.push(AstIssue {
                    severity: Severity::P1,
                    issue_type: "CACHE_NO_EXPIRE".to_string(),
                    file: file_name.clone(),
                    line: line_num,
                    description: "Caffeine/Guava Cache 未设置 expire 或 maximumSize".to_string(),
                });
            }
        }

    // 4. 使用静态编译的正则进行匹配
    let rules = get_rules();
    for rule in &rules {
        // 跳过已特殊处理的规则
        if rule.id == "CACHE_NO_EXPIRE" {
            continue;
        }

        if rule.regex.is_match(&code_without_comments) {
            if let Some(mat) = rule.regex.find(&code_without_comments) {
                let line_num = code_without_comments[..mat.start()].matches('\n').count() + 1;

                // 去重
                let exists = issues.iter().any(|i| i.issue_type == rule.id && i.line == line_num);

                if !exists {
                    issues.push(AstIssue {
                        severity: rule.severity,
                        issue_type: rule.id.to_string(),
                        file: file_name.clone(),
                        line: line_num,
                        description: rule.description.to_string(),
                    });
                }
            }
        }
    }

    issues
}
