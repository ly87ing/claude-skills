# Java Perf v7.0.0 (Rust)

<p align="center">
  <img src="https://img.shields.io/badge/Version-7.0.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Language-Rust-orange" alt="Rust">
  <img src="https://img.shields.io/badge/Size-1.9MB-green" alt="Binary Size">
  <img src="https://img.shields.io/badge/Dependencies-Zero-purple" alt="No Dependencies">
</p>

A Claude Code Plugin for diagnosing Java performance issues using the **Radar-Sniper Architecture v2**.

**Standard Plugin Structure - Marketplace Ready!**

## Architecture

```
Phase 0: 🧠 Knowledge Preload
└── java-perf checklist --symptoms memory

Phase 1: 🛰️ Radar (Zero Cost)
└── java-perf scan --path ./
    ├── Tree-sitter AST (N+1, Nested Loops, ThreadLocal)
    ├── Static Regex Compilation
    └── 20+ Performance Rules (P0/P1)

Phase 2: 🎯 Sniper (LLM Verification)
└── Read source + LSP navigation

Phase 3: 🔬 Forensic (Deep Dive)
└── java-perf jstack/jmap/javap

Phase 4: 📊 Impact Assessment
└── Quantified impact analysis
```

## Installation

### Option 1: Plugin Marketplace (Recommended)

```bash
# Add marketplace and install
/plugin marketplace add ly87ing/java-perf-skill
/plugin install java-perf
```

The `SessionStart` hook will automatically check and install the binary on first session.

### Option 2: Manual Installation

```bash
git clone https://github.com/ly87ing/java-perf-skill.git
cd java-perf-skill
./install.sh
```

Binary + Skill, no registration needed.

### Supported Platforms

- macOS Apple Silicon (arm64)
- macOS Intel (x86_64)
- Linux (x86_64)

## CLI Commands

### Radar Scan (Core)

```bash
# Full project scan (P0 only by default)
java-perf scan --path ./src

# Full scan with P1 warnings
java-perf scan --path ./src --full

# Single file analysis
java-perf analyze --file ./UserService.java
```

### Knowledge Base

```bash
# Get checklist by symptoms
java-perf checklist --symptoms memory,cpu,slow

# List all anti-patterns
java-perf antipatterns
```

### Forensic (JDK Tools)

```bash
# Thread dump analysis
java-perf jstack --pid 12345

# Heap analysis
java-perf jmap --pid 12345

# Bytecode disassembly
java-perf javap --class ./Target.class

# Log analysis
java-perf log --file ./app.log
```

### Utility

```bash
# Project summary
java-perf summary --path ./

# Engine status
java-perf status

# JSON output (any command)
java-perf --json scan --path ./
```

## Detection Rules (45+ Rules)

### P0 Critical (AST-based)

| ID | Description | Engine |
|----|-------------|--------|
| `N_PLUS_ONE` | IO/DB calls inside loops | Tree-sitter |
| `NESTED_LOOP` | Nested loops O(N*M) | Tree-sitter |
| `SYNC_METHOD` | Synchronized on method level | Tree-sitter |
| `THREADLOCAL_LEAK` | ThreadLocal without remove() | Tree-sitter |
| `SLEEP_IN_LOCK` | Thread.sleep() in synchronized | Tree-sitter |
| `LOCK_METHOD_CALL` | ReentrantLock without finally unlock | Tree-sitter |
| `UNBOUNDED_POOL` | Executors.newCachedThreadPool | Regex |
| `UNBOUNDED_CACHE` | static Map without eviction | Regex |
| `FUTURE_GET_NO_TIMEOUT` | Future.get() without timeout | Regex |
| `FLUX_BLOCK` | Flux/Mono.block() blocking call | Regex |
| `FINALIZE_OVERRIDE` | Override finalize() method | Regex |
| `LIKE_LEADING_WILDCARD` | LIKE '%xxx' full table scan | Regex |

### P1 Warning

| ID | Description | Engine |
|----|-------------|--------|
| `STREAM_RESOURCE_LEAK` | Stream created in try block | Tree-sitter |
| `OBJECT_IN_LOOP` | Object allocation inside loops | Regex |
| `SYNC_BLOCK` | Large synchronized block | Regex |
| `NO_TIMEOUT` | HTTP client without timeout | Regex |
| `COMPLETABLE_JOIN` | CompletableFuture.join() | Regex |
| `LOG_STRING_CONCAT` | Logger with string concatenation | Regex |
| `ASYNC_DEFAULT_POOL` | @Async without custom executor | Regex |
| `SCHEDULED_FIXED_RATE` | @Scheduled(fixedRate) backlog risk | Regex |
| `AUTOWIRED_FIELD` | Field injection (prefer constructor) | Regex |
| `SUBSCRIBE_NO_ERROR` | subscribe() without error handler | Regex |
| `FLUX_COLLECT_LIST` | collectList() may cause OOM | Regex |
| `LARGE_ARRAY_ALLOC` | Large array allocation (>1MB) | Regex |
| `STRING_INTERN` | String.intern() metaspace risk | Regex |
| `SELECT_STAR` | SELECT * query | Regex |

## Usage Example

**User:** "系统内存暴涨"

**Claude:**
```
java-perf checklist --symptoms memory
java-perf scan --path ./
→ Found: TraceStore.java:45 THREADLOCAL_LEAK

Read TraceStore.java:40-60
→ Confirmed: ThreadLocal.set() without remove()

📍 Location: TraceStore.java:45
🔍 Root Cause: ThreadLocal not cleaned, thread pool reuses threads
📊 Impact: 1KB/request × 1000 QPS = ~86GB/day memory growth
🔧 Fix: Add finally { threadLocal.remove(); }
```

## Update

```bash
./update.sh
```

## Uninstall

```bash
./uninstall.sh
```

## License

MIT
