//! AST Engine - 双遍语义分析引擎
//!
//! 🛰️ 雷达扫描：检测性能反模式
//!
//! v9.1 架构重构:
//! - AST 规则优先 (tree_sitter_java.rs)
//! - **所有规则已迁移至 Tree-sitter** (v9.1)
//! - 统一规则 ID，消除重复检测
//!
//! 优化点：
//! 1. 使用 thread_local Parser 复用 (v9.1)
//! 2. 过滤注释内容，避免误报
//! 3. 集成 Tree-sitter AST 分析 (v5.0)
//! 4. 并行文件扫描 (rayon) (v5.1)
//! 5. Dockerfile 扫描 (v5.1)
//! 6. 双遍语义引擎 (v8.0)
//! 7. 规则去重，消除 Regex/AST 冲突 (v9.0)
//! 8. 移除所有 Regex 规则，全部使用 Tree-sitter (v9.1)

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

// v9.1: Regex 规则已全部迁移到 tree_sitter_java.rs
// 现在所有 Java 规则都通过 Tree-sitter AST 分析实现

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

/// 全项目雷达扫描 (v9.1 优化架构)
///
/// ## 性能优化 (v9.1):
/// - **thread_local Parser 复用**: 每个线程只初始化一次 Parser
/// - **预编译 Query**: 所有 Tree-sitter 查询在启动时编译一次
///
/// ## 架构说明:
/// 采用两遍扫描架构是必要的，因为 Phase 2 需要 Phase 1 构建的全局符号表：
/// - Phase 1: 并行扫描所有 Java 文件，提取类/字段信息构建全局符号表
/// - Phase 2: 使用全局符号表进行深度分析（如 N+1 检测需要知道变量类型）
///
/// 虽然每个文件被解析两次，但通过 thread_local Parser 复用，
/// 避免了每次调用都创建 Parser 的开销（主要开销是 native 层初始化）。
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
                // v9.1: 所有规则已迁移到 Tree-sitter AST 分析
                // AST Analysis (with Context)
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
            "## 🛰️ 雷达扫描 (v9.1 AST 引擎)\n\n**P0**: {p0_count} | **P1**: {p1_count} | **文件**: {file_count}\n\n"
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
            "## 🛰️ 雷达扫描结果 (v9.1 AST 引擎)\n\n\
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

/// 单文件扫描 (v9.1: 仅使用 Tree-sitter AST 分析)
pub fn scan_source_code(code: &str, file_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut issues = Vec::new();
    let path = Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext == "java" {
        // v9.1: 仅使用 AST 分析（所有 Regex 规则已迁移）
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
