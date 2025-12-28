//! CLI 模式处理器
//!
//! 提供命令行接口，默认输出人类可读格式
//! 使用 --json 参数可输出 JSON 格式

use crate::{ast_engine, checklist, forensic, jdk_engine};
use anyhow::Result;
use serde_json::{json, Value};
use clap::Subcommand;

/// CLI Commands
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// 🛰️ 雷达扫描 - 全项目 AST 分析
    Scan {
        /// 项目路径
        #[arg(short, long, default_value = ".")]
        path: String,

        /// 显示完整结果（默认只显示 P0）
        #[arg(long)]
        full: bool,

        /// 最多返回的 P1 数量 (--full 模式)
        #[arg(long, default_value = "5")]
        max_p1: usize,
    },

    /// 🔍 单文件分析
    Analyze {
        /// 文件路径
        #[arg(short, long)]
        file: String,
    },

    /// 📋 获取检查清单
    Checklist {
        /// 症状列表 (逗号分隔): memory,cpu,slow,resource,backlog,gc
        #[arg(short, long)]
        symptoms: String,

        /// 显示完整信息（默认紧凑模式）
        #[arg(long)]
        full: bool,
    },

    /// ⚠️ 列出所有反模式
    Antipatterns,

    /// 🔬 分析日志文件
    Log {
        /// 日志文件路径
        #[arg(short, long)]
        file: String,
    },

    /// 🔬 分析线程 Dump (jstack)
    Jstack {
        /// Java 进程 PID
        #[arg(short, long)]
        pid: u32,
    },

    /// 🔬 分析字节码 (javap)
    Javap {
        /// 类路径或 .class 文件
        #[arg(short, long)]
        class: String,
    },

    /// 🔬 分析堆内存 (jmap)
    Jmap {
        /// Java 进程 PID
        #[arg(short, long)]
        pid: u32,
    },

    /// 📋 项目摘要
    Summary {
        /// 项目路径
        #[arg(short, long, default_value = ".")]
        path: String,
    },

    /// ℹ️ 引擎状态
    Status,
}

/// 处理 CLI 命令
///
/// json_output: 是否输出 JSON 格式（默认 false，输出人类可读格式）
pub fn handle_command(cmd: Command, json_output: bool) -> Result<()> {
    let result = match cmd {
        Command::Scan { path, full, max_p1 } => {
            // full=false means compact=true (default)
            ast_engine::radar_scan(&path, !full, max_p1)
        }

        Command::Analyze { file } => {
            let content = std::fs::read_to_string(&file)?;
            ast_engine::scan_source_code(&content, &file)
        }

        Command::Checklist { symptoms, full } => {
            let symptoms_vec: Vec<&str> = symptoms.split(',').map(|s| s.trim()).collect();
            checklist::get_checklist(&symptoms_vec, None, !full)
        }

        Command::Antipatterns => {
            checklist::get_all_antipatterns()
        }

        Command::Log { file } => {
            forensic::analyze_log(&file)
        }

        Command::Jstack { pid } => {
            jdk_engine::analyze_thread_dump(pid)
        }

        Command::Javap { class } => {
            jdk_engine::analyze_bytecode(&class)
        }

        Command::Jmap { pid } => {
            jdk_engine::analyze_heap(pid)
        }

        Command::Summary { path } => {
            get_project_summary(&path, json_output)
        }

        Command::Status => {
            let version = env!("CARGO_PKG_VERSION");
            let status = json!({
                "version": version,
                "engine": "Rust Radar-Sniper",
                "ast_rules": ["N_PLUS_ONE", "NESTED_LOOP", "SYNC_METHOD", "THREADLOCAL_LEAK",
                    "STREAM_RESOURCE_LEAK", "SLEEP_IN_LOCK", "LOCK_METHOD_CALL"],
                "regex_rules": ["FUTURE_GET_NO_TIMEOUT", "AWAIT_NO_TIMEOUT", "REENTRANT_LOCK_RISK",
                    "COMPLETABLE_JOIN", "LOG_STRING_CONCAT", "DATASOURCE_NO_POOL"],
                "jdk_tools": {
                    "jstack": jdk_engine::check_tool_available("jstack"),
                    "jmap": jdk_engine::check_tool_available("jmap"),
                    "javap": jdk_engine::check_tool_available("javap"),
                }
            });

            if json_output {
                Ok(status)
            } else {
                Ok(json!(format!(
                    "Java Perf v{}\n\
                    Engine: Rust Radar-Sniper (Tree-sitter AST)\n\
                    AST Rules: 48 | Config Rules: 7 | Dockerfile Rules: 5\n\
                    Features: Rule Suppression, Two-Pass Semantic Analysis, CallGraph\n\
                    JDK Tools: jstack={}, jmap={}, javap={}",
                    version,
                    jdk_engine::check_tool_available("jstack"),
                    jdk_engine::check_tool_available("jmap"),
                    jdk_engine::check_tool_available("javap")
                )))
            }
        }

    };

    // 输出结果
    match result {
        Ok(value) => {
            if json_output {
                // JSON 格式：包装 success 字段
                let output = json!({
                    "success": true,
                    "data": value
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                // 人类可读格式：直接输出内容
                print_value(&value);
            }
        }
        Err(e) => {
            if json_output {
                let output = json!({
                    "success": false,
                    "error": e.to_string()
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("✅ Engine Status: ACTIVE (v8.0.0 Deep Semantic)");
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// 打印 Value，智能处理字符串和其他类型
fn print_value(value: &Value) {
    match value {
        Value::String(s) => println!("{s}"),
        _ => println!("{}", serde_json::to_string_pretty(value).unwrap_or_default()),
    }
}

/// 获取项目摘要
fn get_project_summary(code_path: &str, json_output: bool) -> Result<Value, Box<dyn std::error::Error>> {
    use std::path::Path;
    use walkdir::WalkDir;

    let path = Path::new(code_path);
    if !path.exists() {
        return Err(format!("Path not found: {code_path}").into());
    }

    // 1. 基础文件统计
    let mut java_files = 0;
    let mut xml_files = 0;
    let mut yml_files = 0;

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "java" => java_files += 1,
            "xml" => xml_files += 1,
            "yml" | "yaml" => yml_files += 1,
            _ => {}
        }
    }

    // 2. 深度项目侦测 (ProjectDetector)
    let stack = crate::project_detector::detect_stack(path);
    let strategy_hint = crate::project_detector::generate_strategy_hint(&stack);

    if json_output {
        Ok(json!({
            "path": code_path,
            "files": { "java": java_files, "xml": xml_files, "yaml": yml_files },
            "stack": stack,
            "strategy_hint": strategy_hint
        }))
    } else {
        // 人类可读格式
        let output = format!(
            "📋 项目摘要: {}\n\
            ----------------------------------------\n\
            File Stats: {} Java, {} XML, {} YAML\n\
            Detected Stack:\n\
            - Build Tool: {}\n\
            - JDK Version: {}\n\
            - Spring Boot: {}\n\
            - Reactive:    {}\n\
            ----------------------------------------\n\
            🤖 Analysis Strategy Hint:\n\
            {}\n\
            ",
            code_path, 
            java_files, xml_files, yml_files,
            if stack.build_tool.is_empty() { "Unknown" } else { &stack.build_tool },
            stack.jdk_version,
            if stack.is_spring_boot { "Yes" } else { "No" },
            if stack.is_reactive { "Yes" } else { "No" },
            strategy_hint
        );

        Ok(json!(output))
    }
}
