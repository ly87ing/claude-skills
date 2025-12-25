# Java Performance Diagnostics

<p align="center">
  <img src="https://img.shields.io/badge/Claude-Skill-blue" alt="Claude Skill">
  <img src="https://img.shields.io/badge/MCP-Server-purple" alt="MCP Server">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License">
</p>

A Claude Skill + MCP Server for diagnosing Java performance issues.

## ✨ Features

- **Natural Language Trigger**: Describe your problem, Claude activates automatically
- **Token Efficient**: MCP tools return only relevant data (~93% token savings)
- **Comprehensive Checklist**: 17 categories, 70+ check items
- **Smart Diagnosis**: Symptom combination, priority-based analysis
- **Deep Knowledge**: Each check item includes verification commands and root cause explanations

## 📊 Statistics

| Metric | Count |
|--------|-------|
| MCP Tools | 6 |
| Check Categories | 17 |
| Check Items | 70+ |
| With Verification Commands | 60+ |
| With Root Cause Explanations | 58 |
| Symptom Combinations | 6 |

## 🚀 Quick Start

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

## 🩺 Supported Symptoms

| Type | Param | Examples |
|------|-------|----------|
| Memory | `memory` | OOM, memory spike, leaks |
| CPU | `cpu` | High usage, lock contention |
| Slow Response | `slow` | High latency, timeout |
| GC Pressure | `gc` | Frequent GC, STW |
| Resource | `resource` | Pool full |
| Message Backlog | `backlog` | Queue buildup |

## 🔧 MCP Tools

| Tool | Description |
|------|-------------|
| `get_checklist` | Check items with priority filter |
| `get_diagnosis` | Single symptom diagnosis |
| `get_combined_diagnosis` | Multi-symptom root cause analysis |
| `search_code_patterns` | LSP/Grep search suggestions |
| `get_all_antipatterns` | Anti-pattern quick reference |
| `get_template` | Report template |

## 📋 Check Item Example

```json
{
  "desc": "循环内 IO/计算",
  "verify": "grep -n 'for.*{' 检查内部是否有 dao/rpc 调用",
  "threshold": "N*M > 10000 需优化",
  "fix": "批量查询替代循环查询",
  "why": "循环100次 x 每次10ms = 1秒，这是最常见的性能杀手"
}
```

## 📁 Directory Structure

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
