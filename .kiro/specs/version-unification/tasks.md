# Implementation Plan

## 1. 文档结构整合

- [ ] 1.1 整合 CHANGELOG.md 文件
  - 将 `plugins/java-perf/rust/CHANGELOG.md` 内容合并到 `plugins/java-perf/CHANGELOG.md`
  - 删除 `plugins/java-perf/rust/CHANGELOG.md`
  - 确保合并后的 CHANGELOG 遵循 Keep a Changelog 格式
  - _Requirements: 2.1, 2.3_

- [ ] 1.2 整合 README.md 文件
  - 将 `plugins/java-perf/rust/README.md` 重命名为 `plugins/java-perf/rust/IMPLEMENTATION.md`
  - 移除 IMPLEMENTATION.md 中的版本号引用（标题和 badge）
  - 更新 `plugins/java-perf/README.md` 为插件主文档
  - _Requirements: 2.2, 2.4_

- [ ] 1.3 整合 ROADMAP.md 文件
  - 将 `plugins/java-perf/rust/ROADMAP.md` 移动到 `plugins/java-perf/ROADMAP.md`
  - 移除版本号引用，改为引用 plugin.json
  - _Requirements: 2.5_

- [ ] 1.4 删除根目录 VERSION 文件
  - 删除 `VERSION` 文件
  - 更新根目录 README.md 的版本管理说明
  - _Requirements: 1.2_

## 2. 实现 sync-version.sh 脚本

- [ ] 2.1 创建 sync-version.sh 基础结构
  - 创建 `plugins/java-perf/scripts/sync-version.sh`
  - 实现 plugin.json 版本读取
  - 实现 SemVer 格式验证
  - 实现 --dry-run 参数支持
  - _Requirements: 3.1, 3.2_

- [ ]* 2.2 编写 Property 1 测试：同步一致性
  - **Property 1: Sync Consistency**
  - **Validates: Requirements 3.3, 3.4, 3.5, 3.6**

- [ ] 2.3 实现 Cargo.toml 版本更新
  - 使用 sed 更新 `version = "x.y.z"` 行
  - 处理文件不存在的情况
  - _Requirements: 3.3_

- [ ] 2.4 实现 README.md 版本更新
  - 更新标题中的版本号
  - 更新 badge 中的版本号
  - _Requirements: 3.4_

- [ ] 2.5 实现 marketplace.json 版本更新
  - 使用 jq 或 sed 更新对应插件条目
  - 只更新当前插件的版本
  - _Requirements: 3.5, 5.2_

- [ ]* 2.6 编写 Property 9 测试：插件隔离性
  - **Property 9: Plugin Isolation**
  - **Validates: Requirements 5.2**

- [ ] 2.7 实现根 README.md 版本表更新
  - 更新插件版本表中对应行
  - _Requirements: 3.6_

- [ ] 2.8 实现 CHANGELOG 版本条目验证
  - 检查 CHANGELOG.md 是否包含当前版本条目
  - 缺失时显示警告
  - _Requirements: 9.5_

- [ ]* 2.9 编写 Property 7 测试：CHANGELOG 版本条目
  - **Property 7: CHANGELOG Version Entry**
  - **Validates: Requirements 9.3, 9.4**

- [ ] 2.10 实现输出摘要
  - 显示更新的文件列表
  - 显示警告信息
  - _Requirements: 3.7_

- [ ]* 2.11 编写 Property 6 测试：Dry-run 不可变性
  - **Property 6: Dry-Run Immutability**
  - **Validates: Requirements 8.4**

## 3. Checkpoint - 确保 sync-version.sh 测试通过

- [ ] 3. Checkpoint
  - Ensure all tests pass, ask the user if questions arise.

## 4. 实现 bump-version.sh 脚本

- [ ] 4.1 创建 bump-version.sh 基础结构
  - 创建 `plugins/java-perf/scripts/bump-version.sh`
  - 实现参数解析 (major/minor/patch)
  - 实现当前版本读取
  - _Requirements: 7.1_

- [ ]* 4.2 编写 Property 2 测试：Major 版本升级
  - **Property 2: Version Bump Correctness - Major**
  - **Validates: Requirements 7.2**

- [ ] 4.3 实现 major 版本升级
  - X.Y.Z → (X+1).0.0
  - _Requirements: 7.2_

- [ ]* 4.4 编写 Property 3 测试：Minor 版本升级
  - **Property 3: Version Bump Correctness - Minor**
  - **Validates: Requirements 7.3**

- [ ] 4.5 实现 minor 版本升级
  - X.Y.Z → X.(Y+1).0
  - _Requirements: 7.3_

- [ ]* 4.6 编写 Property 4 测试：Patch 版本升级
  - **Property 4: Version Bump Correctness - Patch**
  - **Validates: Requirements 7.4**

- [ ] 4.7 实现 patch 版本升级
  - X.Y.Z → X.Y.(Z+1)
  - _Requirements: 7.4_

- [ ] 4.8 实现 plugin.json 更新和自动同步
  - 更新 plugin.json version 字段
  - 自动调用 sync-version.sh
  - 显示旧版本和新版本
  - _Requirements: 7.5, 7.6_

## 5. Checkpoint - 确保 bump-version.sh 测试通过

- [ ] 5. Checkpoint
  - Ensure all tests pass, ask the user if questions arise.

## 6. 实现 validate-versions.sh 脚本

- [ ] 6.1 创建 validate-versions.sh 基础结构
  - 创建 `scripts/validate-versions.sh`
  - 实现插件名参数解析
  - 实现 plugin.json 版本读取
  - _Requirements: 6.1_

- [ ]* 6.2 编写 Property 5 测试：验证检测能力
  - **Property 5: Validation Detection**
  - **Validates: Requirements 6.2, 6.3, 6.4, 6.5**

- [ ] 6.3 实现版本一致性检查
  - 检查 Cargo.toml 版本
  - 检查 marketplace.json 版本
  - 检查 README.md badge 版本
  - 检查根 README.md 插件表版本
  - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [ ] 6.4 实现错误报告
  - 报告所有不一致的文件
  - 显示期望版本和实际版本
  - _Requirements: 6.6_

## 7. Checkpoint - 确保 validate-versions.sh 测试通过

- [ ] 7. Checkpoint
  - Ensure all tests pass, ask the user if questions arise.

## 8. 实现 release.sh 脚本

- [ ] 8.1 创建 release.sh 基础结构
  - 创建 `plugins/java-perf/scripts/release.sh`
  - 实现版本一致性预检查
  - 实现工作目录状态检查
  - _Requirements: 4.1_

- [ ]* 8.2 编写 Property 8 测试：Git Tag 格式
  - **Property 8: Git Tag Format**
  - **Validates: Requirements 4.1, 4.2**

- [ ] 8.3 实现 Git Tag 创建
  - 创建格式为 `<plugin-name>-v<version>` 的 Tag
  - 推送 Tag 到远程仓库
  - _Requirements: 4.2, 4.3_

## 9. 更新 CI 工作流

- [ ] 9.1 更新 version-check.yml
  - 使用 validate-versions.sh 替代内联脚本
  - 添加对所有版本文件的检查
  - _Requirements: 6.1_

## 10. 同步当前版本

- [ ] 10.1 确定正确的当前版本
  - 审查 CHANGELOG.md 确定最新发布版本
  - 更新 plugin.json 为正确版本
  - _Requirements: 1.1_

- [ ] 10.2 运行 sync-version.sh 同步所有文件
  - 执行同步脚本
  - 验证所有文件版本一致
  - _Requirements: 3.2_

## 11. Final Checkpoint - 确保所有测试通过

- [ ] 11. Final Checkpoint
  - Ensure all tests pass, ask the user if questions arise.
