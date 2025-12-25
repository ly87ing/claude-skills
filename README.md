# Java Performance Diagnostics

<p align="center">
  <img src="https://img.shields.io/badge/Claude-Skill-blue" alt="Claude Skill">
  <img src="https://img.shields.io/badge/MCP-Server-purple" alt="MCP Server">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License">
</p>

A Claude Skill + MCP Server for diagnosing Java performance issues.

## Features

- **Natural Language Trigger**: Describe your problem, Claude activates automatically
- **Token Efficient**: MCP tools return only relevant data (~93% token savings)
- **Comprehensive Checklist**: 14 categories, 50+ check items
- **Smart Diagnosis**: Symptom mapping, anti-pattern detection

## Quick Start

### 1. Install MCP Server

```bash
cd mcp
npm install
npm run build
```

### 2. Add to Claude Code

```bash
claude mcp add java-perf -- node /path/to/mcp/dist/index.js
```

### 3. Install Skill

```bash
# Global installation
cp -r skill ~/.claude/skills/java-perf

# Or project-specific
cp -r skill /your-project/.agent/skills/java-perf
```

### 4. Use

Simply describe your performance issue:

```
帮我分析一下内存暴涨的问题...
系统响应很慢，CPU占用很高...
消息队列出现大量积压...
```

## Supported Symptoms

| Type | Examples |
|------|----------|
| **Memory** | OOM, memory spike, GC pressure, leaks |
| **CPU** | High usage, infinite loops, lock contention |
| **Slow Response** | High latency, timeout, blocking |
| **Resource Exhaustion** | Connection pool full, thread pool full |
| **Message Backlog** | Queue buildup, consumption lag |

## MCP Tools

| Tool | Description |
|------|-------------|
| `get_checklist` | Get relevant check items by symptom |
| `get_diagnosis` | Quick diagnosis reference |
| `search_code_patterns` | Code search suggestions (LSP/Grep) |
| `get_all_antipatterns` | Anti-pattern reference |

## Directory Structure

```
java-perf-skill/
├── skill/
│   └── SKILL.md          # Claude Skill definition
├── mcp/
│   ├── src/              # MCP server source
│   ├── dist/             # Compiled output
│   └── package.json
├── README.md
└── LICENSE
```

## License

[MIT License](LICENSE)
