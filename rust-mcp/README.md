# Java Perf v5.3.0 (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Version-5.3.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/Size-2.8MB-green" alt="Binary Size">
  <img src="https://img.shields.io/badge/Dependencies-Zero-purple" alt="No Dependencies">
</p>

Java 性能诊断 MCP Server - **零依赖，单二进制**

## 🚀 优势

| 指标 | Node.js (v3.x) | Rust (v5.3) |
|------|---------------|-------------|
| 安装依赖 | Node.js + npm install | **零依赖** |
| 二进制大小 | ~50MB | **1.9MB** |
| 启动时间 | ~500ms | **~5ms** |
| 内存占用 | ~50MB | **~5MB** |

## 📦 安装

### 一键安装

```bash
./install.sh
```

### 手动安装

```bash
# 编译
cargo build --release

# 注册 MCP
claude mcp add java-perf --scope user -- ~/.local/bin/java-perf
```

## 🔧 工具列表

| 工具 | 描述 |
|------|------|
| `radar_scan` | 🛰️ 全项目 AST 扫描 |
| `scan_source_code` | 🛰️ 单文件分析 |
| `analyze_log` | 🔬 日志指纹归类 |
| `analyze_thread_dump` | 🔬 jstack 分析 |
| `get_engine_status` | 引擎状态 |

## 🔍 检测规则 (28+)

### P0 严重

| 规则 | 描述 |
|------|------|
| `N_PLUS_ONE` | 循环内 IO/数据库调用 |
| `NESTED_LOOP` | 嵌套循环 O(N*M) |
| `SYNC_METHOD` | synchronized 方法级锁 |
| `THREADLOCAL_LEAK` | ThreadLocal 未 remove |
| `UNBOUNDED_POOL` | 无界线程池 |
| `UNBOUNDED_CACHE` | 无界缓存 static Map |
| `EXCEPTION_IGNORE` | 空 catch 块 |

### P1 警告

| 规则 | 描述 |
|------|------|
| `OBJECT_IN_LOOP` | 循环内创建对象 |
| `SYNC_BLOCK_LARGE` | synchronized 大代码块 |
| `ATOMIC_SPIN` | Atomic 自旋 |
| `NO_TIMEOUT` | 可能无超时 |
| `BLOCKING_IO` | 同步文件 IO |
| `STRING_CONCAT_LOOP` | 循环内字符串拼接 |

## 🏗️ 架构

```
src/
├── main.rs         # MCP Server 入口 (stdio)
├── mcp.rs          # JSON-RPC 2.0 协议处理
├── ast_engine.rs   # Tree-sitter Java AST 分析
├── forensic.rs     # 日志指纹归类 (流式处理)
└── jdk_engine.rs   # JDK CLI (jstack/javap/jmap)
```

## 📝 使用示例

在 Claude Code 中：

```
帮我分析一下这个项目的性能问题
全面扫描一下代码的性能反模式
分析这个日志文件的异常
```

## License

MIT
