# Java Perf MCP Server

提供 Java 性能问题排查的 MCP 工具。

## 安装

```bash
npm install
npm run build
```

## 添加到 Claude Code

```bash
claude mcp add java-perf -- node /path/to/mcp/dist/index.js
```

## 提供的工具

| 工具 | 说明 | 参数 |
|------|------|------|
| `get_checklist` | 获取检查项 | `symptoms`: memory, cpu, slow, resource, backlog, gc |
| `get_diagnosis` | 快速诊断 | `symptom`: memory, cpu, slow, resource, backlog |
| `search_code_patterns` | 搜索建议 | `symptom`, `preferLsp`, `maxPatterns` |
| `get_all_antipatterns` | 反模式速查 | `category`: all, memory, cpu, io, concurrency |

## Token 节省

| 方式 | Token 消耗 |
|------|-----------|
| 直接读取 CHECKLIST.md | ~3000 tokens |
| MCP 工具返回 | ~200 tokens |
| **节省** | **~93%** |
