# Java Perf v5.3.0 (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Version-5.3.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/Size-1.9MB-green" alt="Binary Size">
  <img src="https://img.shields.io/badge/Dependencies-Zero-purple" alt="No Dependencies">
  <img src="https://img.shields.io/badge/Tools-10-blue" alt="MCP Tools">
</p>

A Claude Skill + MCP Server for diagnosing Java performance issues using the **Radar-Sniper Architecture v2**.

**Now powered by Rust 🦀 for extreme performance!**

## 🏆 Architecture

```
Phase 0: 🧠 Knowledge Preload (LLM Reasoning)
└── Load checklist & antipatterns for guided analysis

Phase 1: 🛰️ Radar (Zero Cost)
└── Rust AST Engine - Millisecond-level full project scan
    ├── Tree-sitter AST (for/while/foreach loops)
    ├── Static Regex Compilation (Lazy)
    └── 20+ Performance Rules (P0/P1)

Phase 2: 🎯 Sniper (LLM Verification + Reasoning)
└── LSP-guided context verification with causal chain analysis

Phase 3: 🔬 Forensic (Deep Dive)
└── JDK CLI Integration - jstack/javap/jmap support

Phase 4: 📊 Impact Assessment (LLM Reasoning)
└── Quantified impact analysis (memory growth, latency multiplier)
```

## 🚀 Advantages

| Metric | Node.js (v3.x) | Rust (v4.0+) |
|--------|---------------|-------------|
| **Install** | Need Node.js | **Zero Dependencies** (Release Download) |
| **Size** | ~50MB | **~1.9MB** (Single Binary) |
| **Startup** | ~500ms | **~5ms** |
| **Scan Speed** | 1000 files / 10s | **1000 files / 0.2s** |

## 📦 Installation

**No Rust environment required!** The script automatically downloads the pre-compiled binary for your platform.

### Quick Install

```bash
git clone https://github.com/ly87ing/java-perf-skill.git
cd java-perf-skill
./install.sh
```

Supported Platforms:
- macOS Apple Silicon (arm64)
- macOS Intel (x86_64)
- Linux (x86_64)

### Update

```bash
./update.sh
```
*Automatically downloads the latest binary from GitHub Releases.*

### Manual Install (From Source)
*Only if you want to build from scratch:*

```bash
cd rust-mcp
cargo build --release
claude mcp add java-perf --scope user -- $(pwd)/target/release/java-perf
```

## 🔧 MCP Tools (10 Tools)

### 📚 Knowledge Base
| Tool | Description |
|------|-------------|
| `get_checklist` | ❓ Get checklist based on symptoms (memory, cpu, slow...) |
| `get_all_antipatterns` | ⚠️ List all 15+ performance anti-patterns |

### 🛰️ Radar (AST Scan)
| Tool | Description |
|------|-------------|
| `radar_scan` | Project-wide AST scan for performance risks |
| `scan_source_code` | Single file AST analysis |
| `get_project_summary` | 📋 Project structure summary (files, packages, tech stack) |

### 🔬 Forensic (Diagnostics)
| Tool | Description |
|------|-------------|
| `analyze_log` | Log fingerprinting & aggregation |
| `analyze_thread_dump` | `jstack` thread analysis |
| `analyze_bytecode` | `javap` bytecode disassembly |
| `analyze_heap` | `jmap -histo` heap analysis |

### ⚙️ System
| Tool | Description |
|------|-------------|
| `get_engine_status` | Check engine & JDK status |

## 🔍 Detection Rules (28+ Rules)

### 🔴 P0 Critical (AST-based)
| ID | Description | Engine |
|----|-------------|--------|
| `N_PLUS_ONE` | IO/DB calls inside for/while/foreach loops | Tree-sitter AST |
| `NESTED_LOOP` | Nested loops (for-for, foreach-foreach, mixed) O(N*M) | Tree-sitter AST |
| `SYNC_METHOD` | Synchronized on method level | Tree-sitter AST |
| `THREADLOCAL_LEAK` | ThreadLocal.set() without remove() | Tree-sitter AST |
| `SLEEP_IN_LOCK` | Thread.sleep() inside synchronized block | Tree-sitter AST |
| `LOCK_METHOD_CALL` | ReentrantLock.lock() without finally unlock | Tree-sitter AST |
| `UNBOUNDED_POOL` | Executors.newCachedThreadPool | Regex |
| `UNBOUNDED_CACHE` | static Map without eviction | Regex |
| `UNBOUNDED_LIST` | static List/Set growing indefinitely | Regex |
| `EMITTER_UNBOUNDED` | Reactor EmitterProcessor (Backpressure) | Regex |
| `FUTURE_GET_NO_TIMEOUT` | Future.get() without timeout (blocks forever) | Regex |
| `AWAIT_NO_TIMEOUT` | await()/acquire() without timeout | Regex |
| `REENTRANT_LOCK_RISK` | ReentrantLock usage (verify unlock in finally) | Regex |

### 🟡 P1 Warning
| ID | Description | Engine |
|----|-------------|--------|
| `STREAM_RESOURCE_LEAK` | Stream/Connection created in try block | Tree-sitter AST |
| `OBJECT_IN_LOOP` | Object allocation inside loops | Regex |
| `SYNC_BLOCK` | Large synchronized block | Regex |
| `ATOMIC_SPIN` | High contention atomic | Regex |
| `NO_TIMEOUT` | HTTP client without timeout | Regex |
| `BLOCKING_IO` | Blocking IO in async context | Regex |
| `SINKS_NO_BACKPRESSURE` | Sinks.many() without handling | Regex |
| `CACHE_NO_EXPIRE` | Cache missing expireAfterWrite | Regex |
| `COMPLETABLE_JOIN` | CompletableFuture.join() without timeout | Regex |
| `LOG_STRING_CONCAT` | Logger with string concatenation (use placeholders) | Regex |
| `DATASOURCE_NO_POOL` | DriverManager.getConnection (no pool) | Regex |

## 📝 Usage Examples

**Diagnosis:**
> "Help me analyze memory leak issues in this project."

**Scanning:**
> "Scan the whole project for performance risks."

**Forensic:**
> "Analyze this thread dump for deadlocks."

## License

MIT
