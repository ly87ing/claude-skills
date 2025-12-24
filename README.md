# Claude Skills

<p align="center">
  <img src="https://img.shields.io/badge/Claude-Skills-blue" alt="Claude Skills">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License">
  <img src="https://img.shields.io/badge/Version-1.0.0-orange" alt="Version">
</p>

A collection of Claude Agent Skills containing reusable, domain-specific capabilities.

## Directory Structure

```
claude-skills/
├── performance-troubleshoot/   # Performance Troubleshooting Skill
│   ├── SKILL.md                # Main File - Diagnostic flows and scenario guidance
│   ├── REFERENCE.md            # Reference - Decision trees, diagnostics tables, tool recommendations
│   ├── CHECKLIST.md            # Detailed Checklist - 6+1 classification index
│   └── TEMPLATE.md             # Report Template - Dynamic metrics template
├── README.md
└── LICENSE
```

## Installation

### Method 1: Install to ~/.claude/skills (Recommended, Global)

```bash
# 1. Clone the repository
git clone https://github.com/ly87ing/claude-skills.git
cd claude-skills

# 2. Create Claude skills directory (if it doesn't exist)
mkdir -p ~/.claude/skills

# 3. Copy skill to Claude global directory
cp -r performance-troubleshoot ~/.claude/skills/
```

### Method 2: Install to Project Directory (Project Specific)

```bash
# 1. Clone the repository
git clone https://github.com/ly87ing/claude-skills.git
cd claude-skills

# 2. Copy to the .agent/skills directory of your target project
mkdir -p /path/to/your-project/.agent/skills
cp -r performance-troubleshoot /path/to/your-project/.agent/skills/
```

> **Note**: You need to restart Claude after installation for the new Skill to be loaded.

## Available Skills

### [performance-troubleshoot](./performance-troubleshoot/)

A Skill for troubleshooting performance and resource issues, featuring automated multi-round analysis.

**Trigger**: Simply describe a performance issue to trigger it automatically.

```
Please help investigate a memory surge, it went from 3GB to 16GB...
The system response is very slow, and CPU usage is high...
There is a massive backlog in the message queue...
```

**Applicable Scenarios**:

| Issue Type | Examples |
|------------|----------|
| **Slow Response** | High latency, low throughput, lock contention |
| **CPU Issues** | High CPU usage, high load, infinite loops |
| **Memory Issues** | Memory spikes, OOM, frequent GC, leaks |
| **Resource Exhaustion** | Connection pool full, thread pool full, handle exhaustion |
| **Service Unavailable** | Downtime, timeouts, high error rates, avalanches |
| **Message Backlog** | Queue buildup, consumption lag, backpressure |
| **Others** | Any other performance issues |

**Features**:

- **Progressive Diagnosis**: Step-by-step information gathering over 3 rounds.
- **Intelligent Decision Tree**: Automatically recommends Symptoms → Diagnosis → Prescriptions.
- **Comprehensive Checklist**: 14 categories with 150+ checkpoints.
- **Tool Recommendations**: arthas, async-profiler, jstack, etc.
- **Anti-Pattern Warnings**: 5 typical error examples.

## Contribution

Issues and Pull Requests are welcome to add new Skills!

## License

[MIT License](LICENSE)

## References

- [Certified Claude Agent Skills](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)
- [Skills Best Practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
