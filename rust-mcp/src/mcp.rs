//! MCP Protocol Handler
//! 
//! 处理 JSON-RPC 2.0 请求/响应

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{ast_engine, forensic, jdk_engine, checklist};

/// JSON-RPC 请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

/// JSON-RPC 响应
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// MCP 错误码定义
/// 遵循 JSON-RPC 2.0 规范: -32000 至 -32099 为服务器定义错误
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum McpErrorCode {
    /// 通用内部错误
    InternalError = -32603,
    /// IO 错误（文件不存在、读取失败等）
    IoError = -32001,
    /// 解析错误（日志解析、AST 解析失败等）
    ParseError = -32002,
    /// 工具不可用（JDK 工具缺失等）
    ToolNotFound = -32003,
    /// 参数无效
    InvalidArgument = -32004,
}

impl McpErrorCode {
    #[allow(dead_code)]
    fn code(&self) -> i32 {
        *self as i32
    }
}

/// MCP 工具定义
fn get_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "get_checklist",
                "description": "❓ 检查清单 - 根据症状返回检查项",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "symptoms": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "症状列表: memory, cpu, slow, resource, backlog, gc"
                        },
                        "priorityFilter": {
                            "type": "string",
                            "description": "优先级过滤: all, P0, P1, P2"
                        },
                        "compact": {
                            "type": "boolean",
                            "default": true,
                            "description": "紧凑模式：只返回检查项描述，省略 verify/fix/why"
                        }
                    },
                    "required": ["symptoms"]
                }
            },
            {
                "name": "get_all_antipatterns",
                "description": "⚠️ 反模式清单 - 所有性能反模式",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "radar_scan",
                "description": "🛰️ 雷达扫描 - 全项目 AST 分析，返回嫌疑点列表 (P0/P1)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "codePath": {
                            "type": "string",
                            "description": "项目根路径"
                        },
                        "compact": {
                            "type": "boolean",
                            "default": true,
                            "description": "紧凑模式：只返回 P0，每个 issue 只含 id/file/line"
                        },
                        "maxP1": {
                            "type": "integer",
                            "default": 5,
                            "description": "最多返回的 P1 数量 (compact=false 时有效)"
                        }
                    },
                    "required": ["codePath"]
                }
            },
            {
                "name": "scan_source_code",
                "description": "🛰️ 单文件 AST 分析",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "源代码内容" },
                        "filePath": { "type": "string", "description": "文件路径" }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "analyze_log",
                "description": "🔬 日志指纹归类分析",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "logPath": { "type": "string", "description": "日志文件路径" }
                    },
                    "required": ["logPath"]
                }
            },
            {
                "name": "analyze_thread_dump",
                "description": "🔬 线程 Dump 分析 (jstack)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pid": { "type": "integer", "description": "Java 进程 PID" }
                    },
                    "required": ["pid"]
                }
            },
            {
                "name": "analyze_bytecode",
                "description": "🔬 字节码反编译 (javap)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "classPath": { "type": "string", "description": "类路径或 .class 文件" }
                    },
                    "required": ["classPath"]
                }
            },
            {
                "name": "analyze_heap",
                "description": "🔬 堆内存分析 (jmap -histo)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pid": { "type": "integer", "description": "Java 进程 PID" }
                    },
                    "required": ["pid"]
                }
            },
            {
                "name": "get_engine_status",
                "description": "获取引擎状态",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "get_project_summary",
                "description": "📋 项目摘要 - 统计文件数/包数/主要依赖，帮助建立上下文",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "codePath": {
                            "type": "string",
                            "description": "项目根路径"
                        }
                    },
                    "required": ["codePath"]
                }
            }
        ]
    })
}

/// 处理 MCP 请求
pub fn handle_request(request: &str) -> Result<String, Box<dyn std::error::Error>> {
    let req: JsonRpcRequest = serde_json::from_str(request)?;
    
    let result = match req.method.as_str() {
        // MCP 协议方法
        "initialize" => handle_initialize(&req.params),
        "notifications/initialized" => return Ok(String::new()), // 无响应
        "tools/list" => Ok(get_tools()),
        "tools/call" => handle_tool_call(&req.params),
        
        // 未知方法
        _ => Err(format!("Unknown method: {}", req.method).into()),
    };
    
    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(value),
            error: None,
            id: req.id,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: e.to_string(),
            }),
            id: req.id,
        },
    };
    
    Ok(serde_json::to_string(&response)?)
}

/// 创建错误响应
#[allow(dead_code)]
pub fn create_error_response(request: &str, error: &str) -> String {
    let id = serde_json::from_str::<JsonRpcRequest>(request)
        .map(|r| r.id)
        .unwrap_or(Value::Null);
    
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
            code: -32603,
            message: error.to_string(),
        }),
        id,
    };
    
    serde_json::to_string(&response).unwrap_or_default()
}

/// 处理 initialize
fn handle_initialize(_params: &Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "java-perf",
            "version": "5.2.0"
        }
    }))
}

/// 获取项目摘要
fn get_project_summary(code_path: &str) -> Result<Value, Box<dyn std::error::Error>> {
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

    // 扫描文件
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
                // 提取包名
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
            },
            "xml" => {
                xml_files += 1;
                // 检测 pom.xml
                if file_name == "pom.xml" {
                    dependencies.insert("Maven".to_string(), true);
                    // 检测常见依赖
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
            },
            "yml" | "yaml" => yml_files += 1,
            "gradle" | "kts" => {
                dependencies.insert("Gradle".to_string(), true);
            },
            _ => {}
        }
    }

    // 生成报告
    let mut report = format!(
        "## 📋 项目摘要: {}\\n\\n\
        **文件统计**:\\n\
        - Java 文件: {}\\n\
        - XML 配置: {}\\n\
        - YAML 配置: {}\\n\\n\
        **包结构** ({} 个包):\\n",
        code_path, java_files, xml_files, yml_files, packages.len()
    );

    // 显示前 10 个包
    for pkg in packages.iter().take(10) {
        report.push_str(&format!("- `{pkg}`\\n"));
    }
    if packages.len() > 10 {
        report.push_str(&format!("- ... 还有 {} 个包\\n", packages.len() - 10));
    }

    if !dependencies.is_empty() {
        report.push_str("\\n**检测到的技术栈**:\\n");
        for dep in dependencies.keys() {
            report.push_str(&format!("- {dep}\\n"));
        }
    }

    Ok(json!(report))
}

/// 处理工具调用
fn handle_tool_call(params: &Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    let params = params.as_ref().ok_or("Missing params")?;
    let tool_name = params.get("name").and_then(|v| v.as_str()).ok_or("Missing tool name")?;
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    
    let result = match tool_name {
        "get_checklist" => {
            let symptoms: Vec<&str> = arguments.get("symptoms")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let priority = arguments.get("priorityFilter")
                .and_then(|v| v.as_str());
            let compact = arguments.get("compact")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            checklist::get_checklist(&symptoms, priority, compact)
        },
        "get_all_antipatterns" => {
            checklist::get_all_antipatterns()
        },
        "radar_scan" => {
            let code_path = arguments.get("codePath")
                .and_then(|v| v.as_str())
                .unwrap_or("./");
            let compact = arguments.get("compact")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let max_p1 = arguments.get("maxP1")
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as usize;
            ast_engine::radar_scan(code_path, compact, max_p1)
        },
        "scan_source_code" => {
            let code = arguments.get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing code")?;
            let file_path = arguments.get("filePath")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown.java");
            ast_engine::scan_source_code(code, file_path)
        },
        "analyze_log" => {
            let log_path = arguments.get("logPath")
                .and_then(|v| v.as_str())
                .ok_or("Missing logPath")?;
            forensic::analyze_log(log_path)
        },
        "analyze_thread_dump" => {
            let pid = arguments.get("pid")
                .and_then(|v| v.as_i64())
                .ok_or("Missing pid")? as u32;
            jdk_engine::analyze_thread_dump(pid)
        },
        "analyze_bytecode" => {
            let class_path = arguments.get("classPath")
                .and_then(|v| v.as_str())
                .ok_or("Missing classPath")?;
            jdk_engine::analyze_bytecode(class_path)
        },
        "analyze_heap" => {
            let pid = arguments.get("pid")
                .and_then(|v| v.as_i64())
                .ok_or("Missing pid")? as u32;
            jdk_engine::analyze_heap(pid)
        },
        "get_engine_status" => {
            Ok(json!({
                "version": "5.3.0",
                "engine": "Rust Radar-Sniper",
                "ast_analyzer": "Tree-sitter + Regex (hybrid)",
                "ast_rules": [
                    "N_PLUS_ONE", "NESTED_LOOP", "SYNC_METHOD", "THREADLOCAL_LEAK", 
                    "STREAM_RESOURCE_LEAK", "SLEEP_IN_LOCK", "LOCK_METHOD_CALL"
                ],
                "regex_rules": [
                    "FUTURE_GET_NO_TIMEOUT", "AWAIT_NO_TIMEOUT", "REENTRANT_LOCK_RISK",
                    "COMPLETABLE_JOIN", "LOG_STRING_CONCAT", "DATASOURCE_NO_POOL"
                ],
                "jdk_tools": {
                    "jstack": jdk_engine::check_tool_available("jstack"),
                    "jmap": jdk_engine::check_tool_available("jmap"),
                    "javap": jdk_engine::check_tool_available("javap"),
                },
                "available_tools": ["radar_scan", "scan_source_code", "analyze_log", "analyze_thread_dump", "analyze_bytecode", "analyze_heap", "get_project_summary"]
            }))
        },
        "get_project_summary" => {
            let code_path = arguments.get("codePath")
                .and_then(|v| v.as_str())
                .unwrap_or("./");
            get_project_summary(code_path)
        },
        _ => Err(format!("Unknown tool: {tool_name}").into()),
    };
    
    match result {
        Ok(content) => Ok(json!({
            "content": [{
                "type": "text",
                "text": content.to_string()
            }]
        })),
        Err(e) => Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Error: {}", e)
            }],
            "isError": true
        })),
    }
}

// ============================================================================
// McpServer 结构体定义 (补全)
// ============================================================================

pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        McpServer
    }

    /// 运行 Server Loop
    pub async fn run<R>(&self, mut input: R) -> anyhow::Result<()> 
    where R: std::io::BufRead {
        use std::io::Write;

        let mut line = String::new();
        loop {
            line.clear();
            if input.read_line(&mut line)? == 0 {
                break; // EOF
            }

            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                match handle_request(trimmed) {
                    Ok(response) => {
                        let _ = std::io::stdout().write_all(response.as_bytes());
                        let _ = std::io::stdout().write_all(b"\n");
                        let _ = std::io::stdout().flush();
                    },
                    Err(e) => {
                        eprintln!("Error handling request: {e}");
                    }
                }
            }
        }
        Ok(())
    }
}
