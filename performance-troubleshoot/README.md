# Java 性能问题排查 Skill

专业的 Java 性能问题排查 Claude Skill，帮助开发者分析和解决各类性能与资源问题。

## 文件结构

| 文件 | 说明 |
|------|------|
| SKILL.md | 核心流程和检查点 |
| CHECKLIST.md | 详细检查清单 |
| REFERENCE.md | 诊断参考资料 |
| TEMPLATE.md | 报告输出模板 |

## 使用方法

当遇到 Java 性能问题时，向 Claude 描述问题即可触发此 Skill：

```
我的服务内存暴涨，请帮我排查
```

然后提供代码目录和辅助物料（日志、Dump 等）。

## 增强功能（可选）

安装 Java LSP 插件可获得更精准的代码分析能力：

```bash
# 在 Claude Code 中执行
/plugin install jdtls-lsp@claude-plugins-official
```

**LSP 增强能力**：
- **Find References**：精准查找方法调用位置
- **Call Hierarchy**：自动构建调用链
- **Go to Definition**：快速跳转到方法定义
- **Diagnostics**：实时代码诊断

> 如未安装 LSP，Skill 会使用基础搜索方式。

## 支持的问题类型

- 响应慢
- CPU 高
- 内存暴涨 / OOM
- GC 频繁
- 连接池/线程池耗尽
- 服务不可用
- 消息积压
