# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [9.5.0] - 2025-12-27

### Added
- **版本同步脚本**: `scripts/sync-version.sh` 确保版本号一致性
- **CI 版本检查**: `.github/workflows/version-check.yml` 自动验证版本同步
- **Import 解析基础**: `extract_imports()` 方法 + `import_query` 预编译

### Fixed
- 修复 `tree_sitter_java.rs` 中 `extract_imports` 重复定义问题
- 统一 SKILL.md 版本号至 v9.4.0

### Technical
- 48 个测试用例全部通过
- 代码清理，移除重复函数

## [9.4.0] - 2025-12-27

### Added
- **CallGraph 污点分析**: Phase 1 同时构建 SymbolTable + CallGraph
- **N+1 验证增强**: `NPlusOneHandler` 使用 `trace_to_layer` 验证调用链
- **serde_yaml 配置解析**: 结构化 Spring 配置分析
- **Query 外部化**: `include_str!` 加载 `resources/queries/*.scm`

### Changed
- SymbolTable 并行合并 (Rayon reduce)
- 版本号动态获取 `env!("CARGO_PKG_VERSION")`
- `RuleContext` 扩展 `call_graph` 字段

### Technical
- 48 个测试用例全部通过

## [9.3.0] - 2025-12-26

### Added
- **RuleHandler trait**: 多态分发替代巨型 match
- **预编译 Query**: 一次编译，多次使用

## [8.0.0] - 2025-12-26

### Added
- **Two-Pass 架构**: Indexing → Analysis
- **语义分析**: SymbolTable 跨文件类型追踪
- **动态 Skill 策略**: 基于项目技术栈

## [6.0.0] - 2025-12-26

### Changed
- **纯 CLI + Skill 模式**: 移除 MCP 依赖，简化分发和使用

### Removed
- `rust/src/mcp.rs` - MCP Server 实现
- `--with-mcp` 安装参数

### Architecture
```
v5.x (MCP 模式)                     v6.0.0 (CLI + Skill)
├── 需要 MCP 注册                   ├── 只需二进制 + Skill
├── mcp__java-perf__scan            ├── java-perf scan
├── JSON 输出需解析                 ├── Markdown 直接可读
└── 配置复杂                        └── 零配置
```

### Performance
| 场景 | v5.x (JSON) | v6.0.0 (Markdown) | 节省 |
|------|-------------|-------------------|------|
| scan 无问题 | ~150 tokens | ~80 tokens | 47% |
| scan 有问题 | ~300 tokens | ~150 tokens | 50% |
| checklist | ~200 tokens | ~100 tokens | 50% |

## [5.3.0] - 2025-12-26

### Added
- 新增 8 条检测规则

## [5.2.0] - 2025-12-25

### Added
- Tree-sitter AST 分析引擎
- N+1、嵌套循环、ThreadLocal 泄漏检测

## [4.0.0] - 2025-12-24

### Changed
- Rust 重写实现
