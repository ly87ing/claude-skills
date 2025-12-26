//! CLI 模式处理器
//!
//! 提供命令行接口，默认输出人类可读格式
//! 使用 --json 参数可输出 JSON 格式

use crate::{ast_engine, checklist, forensic, jdk_engine, Command};
use anyhow::Result;
use serde_json::{json, Value};

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
            let status = json!({
                "version": "6.0.0",
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
                // 人类可读格式
                Ok(json!(format!(
                    "Java Perf v6.0.0\n\
                    Engine: Rust Radar-Sniper (Tree-sitter + Regex)\n\
                    AST Rules: 7 | Regex Rules: 6\n\
                    JDK Tools: jstack={}, jmap={}, javap={}",
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
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// 打印 Value，智能处理字符串和其他类型
fn print_value(value: &Value) {
    match value {
        Value::String(s) => println!("{}", s),
        _ => println!("{}", serde_json::to_string_pretty(value).unwrap_or_default()),
    }
}

/// 获取项目摘要
fn get_project_summary(code_path: &str, json_output: bool) -> Result<Value, Box<dyn std::error::Error>> {
    use std::collections::{HashMap, HashSet};
    use std::path::Path;
    use walkdir::WalkDir;

    let path = Path::new(code_path);
    if !path.exists() {
        return Err(format!("Path not found: {code_path}").into());
    }

    let mut java_files = 0;
    let mut xml_files = 0;
    let mut yml_files = 0;
    let mut packages: HashSet<String> = HashSet::new();
    let mut dependencies: HashMap<String, bool> = HashMap::new();

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match ext {
            "java" => {
                java_files += 1;
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    for line in content.lines().take(10) {
                        if line.starts_with("package ") {
                            let pkg = line.trim_start_matches("package ")
                                .trim_end_matches(';')
                                .trim();
                            packages.insert(pkg.to_string());
                            break;
                        }
                    }
                }
            }
            "xml" => {
                xml_files += 1;
                if file_name == "pom.xml" {
                    dependencies.insert("Maven".to_string(), true);
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        if content.contains("spring-boot") {
                            dependencies.insert("Spring Boot".to_string(), true);
                        }
                        if content.contains("mybatis") {
                            dependencies.insert("MyBatis".to_string(), true);
                        }
                        if content.contains("reactor") || content.contains("webflux") {
                            dependencies.insert("Reactor/WebFlux".to_string(), true);
                        }
                        if content.contains("jedis") || content.contains("lettuce") {
                            dependencies.insert("Redis".to_string(), true);
                        }
                        if content.contains("kafka") {
                            dependencies.insert("Kafka".to_string(), true);
                        }
                    }
                }
            }
            "yml" | "yaml" => yml_files += 1,
            "gradle" | "kts" => {
                dependencies.insert("Gradle".to_string(), true);
            }
            _ => {}
        }
    }

    if json_output {
        Ok(json!({
            "path": code_path,
            "files": { "java": java_files, "xml": xml_files, "yaml": yml_files },
            "packages": packages.into_iter().collect::<Vec<_>>(),
            "dependencies": dependencies.keys().cloned().collect::<Vec<_>>()
        }))
    } else {
        // 人类可读格式
        let deps: Vec<_> = dependencies.keys().cloned().collect();
        let pkgs: Vec<_> = packages.into_iter().collect();
        let pkg_count = pkgs.len();
        let pkg_display: Vec<_> = pkgs.into_iter().take(5).collect();

        let mut output = format!(
            "📋 项目摘要: {}\n\
            Files: {} Java, {} XML, {} YAML\n",
            code_path, java_files, xml_files, yml_files
        );

        if pkg_count > 0 {
            output.push_str(&format!("Packages: {} total", pkg_count));
            if pkg_count > 5 {
                output.push_str(&format!(" (showing first 5: {})", pkg_display.join(", ")));
            } else {
                output.push_str(&format!(" ({})", pkg_display.join(", ")));
            }
            output.push('\n');
        }

        output.push_str(&format!(
            "Tech: {}",
            if deps.is_empty() { "None detected".to_string() } else { deps.join(", ") }
        ));

        Ok(json!(output))
    }
}
