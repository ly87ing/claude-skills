# Requirements Document

## Introduction

本功能旨在为 dev-skills 插件市场建立符合 Claude Code 官方规范的版本管理机制。

### Claude Code 官方版本管理建议

根据 Claude Code 官方文档和最佳实践：

1. **基于 Manifest 的版本号**：
   - 每个插件必须包含 `plugin.json`，其中 `version` 字段是核心标识
   - 市场必须包含 `marketplace.json`，包含版本元数据

2. **Git-native 工作流**：
   - 使用 Git Tags 标记发布版本
   - 遵循语义化版本（SemVer）规范
   - `/plugin update` 通过拉取 Git 仓库最新状态来同步

3. **去中心化市场模式**：
   - 插件市场本质上是一个 Git 仓库
   - 类似 Homebrew Taps 或 CocoaPods Specs 的模式

### 当前问题分析

1. **版本来源混乱**：
   - 根目录 `VERSION` 文件 (8.1.0)
   - `plugin.json` (8.1.0)
   - `Cargo.toml` (8.1.0)
   - `plugins/java-perf/README.md` (8.1.0)
   - `plugins/java-perf/rust/README.md` (9.5.0) ← 不一致
   - `plugins/java-perf/rust/CHANGELOG.md` (9.5.0) ← 不一致
   - `plugins/java-perf/rust/ROADMAP.md` (9.5.0) ← 不一致

2. **文件结构混乱**：
   - 两个 CHANGELOG.md（插件根目录 vs rust/ 目录）
   - 两个 README.md（插件根目录 vs rust/ 目录）
   - ROADMAP.md 只在 rust/ 目录

3. **缺乏清晰的文件职责划分**：
   - 插件级文档 vs Rust 实现级文档 职责不清

### 设计目标

- `plugin.json` 作为插件版本的权威来源（符合官方规范）
- 整合重复的文档文件
- Git Tags 与 `plugin.json` 版本保持同步
- 自动化脚本确保所有版本引用一致

## Glossary

- **Marketplace**: Claude Code 插件市场，本质是一个 Git 仓库
- **Plugin**: 独立的 Claude Code 插件，如 java-perf
- **plugin.json**: 插件元数据文件，`version` 字段是核心版本标识
- **marketplace.json**: 市场定义文件，列出所有可用插件
- **SemVer**: 语义化版本规范 (MAJOR.MINOR.PATCH)
- **Git Tag**: Git 标签，用于标记发布版本

## Requirements

### Requirement 1

**User Story:** As a plugin developer, I want plugin.json to be the single source of truth for version, so that it aligns with Claude Code official specification.

#### Acceptance Criteria

1. THE plugin.json version field SHALL serve as the authoritative version source for each plugin
2. THE root VERSION file SHALL be removed to eliminate version source confusion
3. WHEN querying plugin version THEN the system SHALL read from plugin.json
4. EACH plugin SHALL maintain version in its `.claude-plugin/plugin.json` file

### Requirement 2

**User Story:** As a plugin developer, I want consolidated documentation files, so that there is no confusion about which file to update.

#### Acceptance Criteria

1. EACH plugin SHALL have a single CHANGELOG.md at the plugin root directory (e.g., `plugins/java-perf/CHANGELOG.md`)
2. EACH plugin SHALL have a single README.md at the plugin root directory (e.g., `plugins/java-perf/README.md`)
3. THE rust/CHANGELOG.md file SHALL be removed or merged into the plugin-level CHANGELOG.md
4. THE rust/README.md file SHALL be converted to a technical implementation guide without version references
5. THE rust/ROADMAP.md file SHALL be moved to plugin root or merged into CHANGELOG.md

### Requirement 3

**User Story:** As a plugin developer, I want a sync script that propagates version from plugin.json to all related files, so that versions stay consistent.

#### Acceptance Criteria

1. EACH plugin directory SHALL contain a `scripts/sync-version.sh` script
2. WHEN the sync script executes THEN the script SHALL read version from plugin.json
3. WHEN the sync script executes THEN the script SHALL update rust/Cargo.toml version field
4. WHEN the sync script executes THEN the script SHALL update README.md title and badge versions
5. WHEN the sync script executes THEN the script SHALL update the plugin entry in root marketplace.json
6. WHEN the sync script executes THEN the script SHALL update the root README.md plugin version table
7. WHEN the sync script completes THEN the script SHALL display a summary of updated files

### Requirement 4

**User Story:** As a plugin developer, I want Git Tags to be synchronized with plugin versions, so that releases are properly tracked following Claude Code best practices.

#### Acceptance Criteria

1. WHEN a new version is released THEN a Git Tag SHALL be created in format `<plugin-name>-v<version>`
2. THE Git Tag version SHALL match the plugin.json version exactly
3. WHEN the release script executes THEN the script SHALL create and push the Git Tag automatically
4. WHEN listing releases THEN the system SHALL use Git Tags as the source of truth

### Requirement 5

**User Story:** As a marketplace maintainer, I want marketplace.json to reflect actual plugin versions, so that users see correct information when browsing.

#### Acceptance Criteria

1. THE marketplace.json plugins array SHALL list each plugin with its current version
2. WHEN a plugin sync script runs THEN the script SHALL update only its own entry in marketplace.json
3. WHEN marketplace.json is updated THEN the version SHALL match the plugin.json version exactly

### Requirement 6

**User Story:** As a CI/CD pipeline, I want automatic version validation on pull requests, so that version mismatches are caught before merge.

#### Acceptance Criteria

1. WHEN a pull request is created THEN the CI workflow SHALL verify version consistency for each modified plugin
2. IF Cargo.toml version does not match plugin.json version THEN the CI workflow SHALL fail with descriptive error
3. IF marketplace.json plugin entry does not match plugin.json version THEN the CI workflow SHALL fail with descriptive error
4. IF README.md badge version does not match plugin.json version THEN the CI workflow SHALL fail with descriptive error
5. IF root README.md plugin table version does not match plugin.json version THEN the CI workflow SHALL fail with descriptive error
6. WHEN validation fails THEN the CI workflow SHALL report all mismatched files with expected and actual versions

### Requirement 7

**User Story:** As a plugin developer, I want a version bump command, so that I can easily increment versions following SemVer.

#### Acceptance Criteria

1. EACH plugin directory SHALL contain a `scripts/bump-version.sh` script
2. WHEN the bump script receives "major" argument THEN the script SHALL increment major version and reset minor and patch to zero
3. WHEN the bump script receives "minor" argument THEN the script SHALL increment minor version and reset patch to zero
4. WHEN the bump script receives "patch" argument THEN the script SHALL increment patch version
5. WHEN the bump script completes THEN the script SHALL update plugin.json and run sync-version.sh automatically
6. WHEN the bump script completes THEN the script SHALL display the old and new version numbers

### Requirement 8

**User Story:** As a developer, I want clear error handling and dry-run support, so that I can safely manage versions.

#### Acceptance Criteria

1. IF plugin.json does not exist THEN the sync script SHALL exit with error code 1 and display descriptive error message
2. IF plugin.json version field is missing or invalid THEN the sync script SHALL exit with error code 1 and display expected format
3. IF a target file is missing THEN the sync script SHALL warn and continue processing remaining files
4. WHEN the sync script runs with --dry-run flag THEN the script SHALL display proposed changes without modifying files
5. WHEN the sync script completes THEN the script SHALL display summary of updated files and any warnings

### Requirement 9

**User Story:** As a developer, I want CHANGELOG.md to follow a consistent format, so that version history is clear and maintainable.

#### Acceptance Criteria

1. THE CHANGELOG.md SHALL follow Keep a Changelog format (https://keepachangelog.com)
2. EACH version entry SHALL include the version number and release date
3. THE latest version in CHANGELOG.md SHALL match the plugin.json version
4. WHEN a new version is released THEN the CHANGELOG.md SHALL have a corresponding entry
5. THE sync script SHALL validate that CHANGELOG.md contains an entry for the current version
