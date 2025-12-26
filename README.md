# Java Perf v3.1.0 (Radar-Sniper)

<p align="center">
  <img src="https://img.shields.io/badge/Version-3.1.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/Claude-Skill-purple" alt="Claude Skill">
  <img src="https://img.shields.io/badge/MCP-15_Tools-green" alt="MCP Tools">
  <img src="https://img.shields.io/badge/License-MIT-yellow" alt="MIT License">
</p>

A Claude Skill + MCP Server for diagnosing Java performance issues using the **Radar-Sniper Architecture**.

## 🏆 Architecture

```
Phase 1: 🛰️ Radar (0 Token)
└── Tree-sitter AST - Full project scan, mark suspects

Phase 2: 🎯 Sniper (LSP)
└── Jump to marked locations only, verify context

Phase 3: 🔬 Forensic (Optional)
└── JDK CLI - jstack/javap/jmap deep analysis
```

## 📊 Statistics

| Metric | Count |
|--------|-------|
| MCP Tools | **15** |
| Check Items | **71** |
| AST Detection Patterns | 5 |
| JDK CLI Commands | 3 |

## 🚀 Quick Start

### Install

```bash
git clone https://github.com/ly87ing/java-perf-skill.git
cd java-perf-skill
./install.sh
```

### Update

```bash
./update.sh
```

### Uninstall

```bash
./uninstall.sh
```

## 🔧 MCP Tools

### 🛰️ Radar (AST Analysis)

| Tool | Function |
|------|----------|
| `radar_scan` | Full project scan |
| `scan_source_code` | Single file analysis |

### 🔬 Forensic (JDK CLI)

| Tool | Function |
|------|----------|
| `analyze_thread_dump` | Thread dump analysis |
| `analyze_bytecode` | Bytecode disassembly |
| `analyze_heap` | Heap memory statistics |

### 🚀 All-in-One

| Tool | Function |
|------|----------|
| `java_perf_investigation` | Complete diagnosis |
| `diagnose_all` | Checklist + Diagnosis |

## 🩺 Usage

Simply describe your performance issue:

```
帮我分析一下内存暴涨的问题...
全面扫描一下项目的性能问题...
分析一下线程死锁原因...
```

## 📁 Structure

```
java-perf-skill/
├── skill/SKILL.md      # Radar-Sniper protocol
├── mcp/src/
│   ├── index.ts        # 15 MCP tools
│   ├── utils/
│   │   ├── ast-engine.ts   # Tree-sitter radar
│   │   ├── jdk-engine.ts   # JDK forensic
│   │   ├── forensic.ts     # Log analysis
│   │   └── audit.ts        # Regex audit
│   └── checklist-data.ts   # 71 check items
├── install.sh
├── update.sh
└── uninstall.sh
```

## License

[MIT License](LICENSE)
