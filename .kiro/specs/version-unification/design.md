# Design Document: Version Unification

## Overview

本设计文档描述了 dev-skills 插件市场的版本统一管理机制。核心目标是建立以 `plugin.json` 为权威来源的版本管理体系，通过自动化脚本确保所有版本引用保持一致。

### 设计原则

1. **Single Source of Truth**: `plugin.json` 是插件版本的唯一权威来源
2. **Git-native**: 使用 Git Tags 标记发布版本，符合 Claude Code 官方建议
3. **自动化优先**: 通过脚本自动同步版本，减少人为错误
4. **向后兼容**: 保持与现有 Claude Code 插件规范的兼容性

## Architecture

```
dev-skills/
├── .claude-plugin/
│   └── marketplace.json          # 市场定义（包含插件版本列表）
├── plugins/
│   └── java-perf/
│       ├── .claude-plugin/
│       │   └── plugin.json       # 🔑 版本权威来源
│       ├── scripts/
│       │   ├── sync-version.sh   # 版本同步脚本
│       │   ├── bump-version.sh   # 版本升级脚本
│       │   └── release.sh        # 发布脚本（创建 Git Tag）
│       ├── rust/
│       │   ├── Cargo.toml        # ← 同步目标
│       │   └── IMPLEMENTATION.md # 技术实现文档（无版本号）
│       ├── README.md             # ← 同步目标（标题 + badge）
│       ├── CHANGELOG.md          # ← 验证目标（需包含当前版本条目）
│       └── ROADMAP.md            # 路线图（可选）
├── README.md                     # ← 同步目标（插件版本表）
└── scripts/
    └── validate-versions.sh      # CI 版本验证脚本
```

### 版本流向

```
plugin.json (权威来源)
    │
    ├─── sync-version.sh ───┬──→ rust/Cargo.toml
    │                       ├──→ README.md (标题 + badge)
    │                       ├──→ marketplace.json (对应条目)
    │                       └──→ 根 README.md (插件表)
    │
    └─── release.sh ────────────→ Git Tag (java-perf-v8.1.0)
```

## Components and Interfaces

### 1. sync-version.sh

版本同步脚本，从 `plugin.json` 读取版本并更新所有目标文件。

```bash
#!/bin/bash
# 用法: ./scripts/sync-version.sh [--dry-run]

# 接口
# 输入: plugin.json version 字段
# 输出: 更新后的文件列表或 dry-run 报告
# 退出码: 0 成功, 1 错误
```

**功能：**
- 读取 `plugin.json` 中的 version 字段
- 验证版本格式（SemVer）
- 更新 `rust/Cargo.toml` 的 version 字段
- 更新 `README.md` 的标题和 badge 版本
- 更新根目录 `marketplace.json` 中对应插件的版本
- 更新根目录 `README.md` 的插件版本表
- 验证 `CHANGELOG.md` 包含当前版本条目
- 支持 `--dry-run` 模式

### 2. bump-version.sh

版本升级脚本，按 SemVer 规范升级版本号。

```bash
#!/bin/bash
# 用法: ./scripts/bump-version.sh <major|minor|patch>

# 接口
# 输入: 升级类型 (major/minor/patch)
# 输出: 旧版本 → 新版本
# 副作用: 更新 plugin.json，自动调用 sync-version.sh
```

**功能：**
- 读取当前版本
- 按类型升级版本号
- 更新 `plugin.json`
- 自动调用 `sync-version.sh`

### 3. release.sh

发布脚本，创建 Git Tag 并推送。

```bash
#!/bin/bash
# 用法: ./scripts/release.sh

# 接口
# 输入: plugin.json version
# 输出: Git Tag (java-perf-v<version>)
# 前置条件: 所有版本已同步，工作目录干净
```

**功能：**
- 验证版本一致性
- 创建 Git Tag（格式：`<plugin-name>-v<version>`）
- 推送 Tag 到远程仓库

### 4. validate-versions.sh

CI 验证脚本，检查所有版本引用的一致性。

```bash
#!/bin/bash
# 用法: ./scripts/validate-versions.sh [plugin-name]

# 接口
# 输入: 可选的插件名（默认验证所有插件）
# 输出: 验证结果报告
# 退出码: 0 全部一致, 1 存在不一致
```

**功能：**
- 读取 `plugin.json` 版本
- 检查 `Cargo.toml` 版本
- 检查 `marketplace.json` 版本
- 检查 `README.md` badge 版本
- 检查根 `README.md` 插件表版本
- 报告所有不一致的文件

## Data Models

### plugin.json 结构

```json
{
  "name": "java-perf",
  "version": "8.1.0",  // 🔑 权威版本来源
  "description": "...",
  "author": { "name": "...", "url": "..." },
  "repository": "...",
  "license": "MIT",
  "keywords": [...],
  "hooks": "./hooks/hooks.json",
  "skills": "./skills/"
}
```

### marketplace.json 结构

```json
{
  "name": "dev-skills",
  "owner": { "name": "...", "url": "..." },
  "description": "...",
  "repository": "...",
  "plugins": [
    {
      "name": "java-perf",
      "version": "8.1.0",  // ← 从 plugin.json 同步
      "description": "...",
      "source": "./plugins/java-perf",
      "license": "MIT"
    }
  ]
}
```

### 版本格式

遵循 SemVer 规范：`MAJOR.MINOR.PATCH`

- **MAJOR**: 不兼容的 API 变更
- **MINOR**: 向后兼容的功能新增
- **PATCH**: 向后兼容的问题修复

正则验证：`^[0-9]+\.[0-9]+\.[0-9]+$`

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Sync Consistency

*For any* valid version in plugin.json, after running sync-version.sh, all target files (Cargo.toml, README.md badge, marketplace.json entry, root README.md table) SHALL contain the same version string.

**Validates: Requirements 3.3, 3.4, 3.5, 3.6**

### Property 2: Version Bump Correctness - Major

*For any* version X.Y.Z, when bump-version.sh receives "major" argument, the resulting version SHALL be (X+1).0.0.

**Validates: Requirements 7.2**

### Property 3: Version Bump Correctness - Minor

*For any* version X.Y.Z, when bump-version.sh receives "minor" argument, the resulting version SHALL be X.(Y+1).0.

**Validates: Requirements 7.3**

### Property 4: Version Bump Correctness - Patch

*For any* version X.Y.Z, when bump-version.sh receives "patch" argument, the resulting version SHALL be X.Y.(Z+1).

**Validates: Requirements 7.4**

### Property 5: Validation Detection

*For any* version mismatch between plugin.json and any target file, validate-versions.sh SHALL detect the mismatch and report it with both expected and actual versions.

**Validates: Requirements 6.2, 6.3, 6.4, 6.5**

### Property 6: Dry-Run Immutability

*For any* execution of sync-version.sh with --dry-run flag, no files SHALL be modified (file checksums remain unchanged).

**Validates: Requirements 8.4**

### Property 7: CHANGELOG Version Entry

*For any* version in plugin.json, CHANGELOG.md SHALL contain an entry with that version number.

**Validates: Requirements 9.3, 9.4**

### Property 8: Git Tag Format

*For any* plugin release, the Git Tag SHALL follow the format `<plugin-name>-v<version>` where version matches plugin.json exactly.

**Validates: Requirements 4.1, 4.2**

### Property 9: Plugin Isolation

*For any* plugin sync operation, only that plugin's entry in marketplace.json SHALL be modified; other plugin entries SHALL remain unchanged.

**Validates: Requirements 5.2**

## Error Handling

### sync-version.sh 错误处理

| 错误场景 | 处理方式 | 退出码 |
|---------|---------|--------|
| plugin.json 不存在 | 显示错误信息，退出 | 1 |
| version 字段缺失 | 显示错误信息，退出 | 1 |
| version 格式无效 | 显示期望格式，退出 | 1 |
| 目标文件不存在 | 警告并继续处理其他文件 | 0 |
| 文件写入失败 | 报告错误并继续 | 0 (带警告) |
| CHANGELOG 缺少版本条目 | 警告（不阻止同步） | 0 (带警告) |

### bump-version.sh 错误处理

| 错误场景 | 处理方式 | 退出码 |
|---------|---------|--------|
| 参数无效 | 显示用法说明 | 1 |
| plugin.json 不存在 | 显示错误信息 | 1 |
| 当前版本格式无效 | 显示错误信息 | 1 |

### validate-versions.sh 错误处理

| 错误场景 | 处理方式 | 退出码 |
|---------|---------|--------|
| 插件不存在 | 显示错误信息 | 1 |
| 版本不一致 | 报告所有不一致项 | 1 |
| 所有版本一致 | 显示成功信息 | 0 |

## Testing Strategy

### 测试框架

- **Shell 脚本测试**: 使用 [bats-core](https://github.com/bats-core/bats-core) 进行 Bash 脚本测试
- **Property-Based Testing**: 使用 bats 结合随机版本号生成进行属性测试

### 单元测试

1. **版本解析测试**
   - 有效版本格式解析
   - 无效版本格式拒绝
   - 边界值测试（0.0.0, 999.999.999）

2. **版本升级测试**
   - major 升级逻辑
   - minor 升级逻辑
   - patch 升级逻辑

3. **文件更新测试**
   - Cargo.toml 版本更新
   - README.md badge 更新
   - marketplace.json 条目更新

### Property-Based Tests

每个属性测试将使用随机生成的版本号进行验证：

```bash
# 示例：Property 1 测试
# 生成随机版本号，运行 sync，验证所有文件版本一致
```

测试配置：
- 最小迭代次数：100
- 版本号范围：0-999 for each component

### 集成测试

1. **完整工作流测试**
   - bump → sync → validate → release 完整流程
   
2. **CI 模拟测试**
   - 模拟 PR 触发版本验证

### 测试标注格式

每个 property-based test 将使用以下格式标注：

```bash
# **Feature: version-unification, Property 1: Sync Consistency**
@test "sync-version.sh maintains version consistency across all files" {
  # test implementation
}
```
