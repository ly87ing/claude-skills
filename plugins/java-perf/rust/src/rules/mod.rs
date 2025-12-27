//! 规则注册中心 (Rule Registry)
//!
//! v9.0 统一规则管理架构
//!
//! 设计目标:
//! 1. 单一数据源 - 所有规则在此定义
//! 2. 支持多种检测器 - AST, Regex, Config
//! 3. 规则抑制 - 支持注解和注释抑制
//! 4. 文档生成 - 可从规则定义生成文档

use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

pub mod definitions;
pub mod suppression;

/// 规则严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// P0 - 严重问题，必须修复
    P0,
    /// P1 - 警告，建议修复
    P1,
}

/// 规则类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    /// N+1 查询、嵌套循环等
    Performance,
    /// 锁、线程安全、死锁风险
    Concurrency,
    /// 内存泄露、GC 压力
    Memory,
    /// Spring 框架相关
    Spring,
    /// 响应式编程 (Reactor/WebFlux)
    Reactive,
    /// 资源管理、连接池
    Resource,
    /// 配置问题
    Config,
    /// 异常处理
    Exception,
    /// 数据库/SQL
    Database,
    /// GraalVM Native Image
    GraalVM,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Performance => "性能",
            Category::Concurrency => "并发",
            Category::Memory => "内存",
            Category::Spring => "Spring",
            Category::Reactive => "响应式",
            Category::Resource => "资源",
            Category::Config => "配置",
            Category::Exception => "异常",
            Category::Database => "数据库",
            Category::GraalVM => "GraalVM",
        }
    }
}

/// 规则定义
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// 规则唯一标识符
    pub id: &'static str,
    /// 规则类别
    pub category: Category,
    /// 严重级别
    pub severity: Severity,
    /// 简短描述
    pub description: &'static str,
    /// 详细说明 (为什么是问题)
    pub rationale: &'static str,
    /// 修复建议
    pub fix_suggestion: &'static str,
    /// 检测器类型
    pub detector: DetectorType,
    /// 是否默认启用
    pub enabled_by_default: bool,
}

/// 检测器类型
#[derive(Debug, Clone)]
pub enum DetectorType {
    /// AST 检测 (Tree-sitter query)
    Ast {
        query: &'static str,
        /// 需要特殊处理逻辑的规则
        handler: Option<&'static str>,
    },
    /// 正则表达式检测
    Regex {
        pattern: &'static str,
    },
    /// 配置文件检测
    Config {
        key: &'static str,
        simple_key: &'static str,
    },
}

/// 规则注册表
pub struct RuleRegistry {
    rules: HashMap<&'static str, RuleDefinition>,
    by_category: HashMap<Category, Vec<&'static str>>,
}

impl RuleRegistry {
    /// 创建规则注册表
    pub fn new() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
            by_category: HashMap::new(),
        };

        // 注册所有规则
        for rule in definitions::all_rules() {
            registry.register(rule);
        }

        registry
    }

    /// 注册规则
    fn register(&mut self, rule: RuleDefinition) {
        let id = rule.id;
        let category = rule.category;

        self.rules.insert(id, rule);
        self.by_category
            .entry(category)
            .or_default()
            .push(id);
    }

    /// 获取规则定义
    pub fn get(&self, id: &str) -> Option<&RuleDefinition> {
        self.rules.get(id)
    }

    /// 获取所有规则
    pub fn all(&self) -> impl Iterator<Item = &RuleDefinition> {
        self.rules.values()
    }

    /// 获取某类别的所有规则
    pub fn by_category(&self, category: Category) -> Vec<&RuleDefinition> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.rules.get(*id)).collect())
            .unwrap_or_default()
    }

    /// 获取所有启用的规则
    pub fn enabled(&self) -> impl Iterator<Item = &RuleDefinition> {
        self.rules.values().filter(|r| r.enabled_by_default)
    }

    /// 获取 AST 规则
    pub fn ast_rules(&self) -> Vec<&RuleDefinition> {
        self.rules.values()
            .filter(|r| matches!(r.detector, DetectorType::Ast { .. }))
            .collect()
    }

    /// 获取 Regex 规则
    pub fn regex_rules(&self) -> Vec<&RuleDefinition> {
        self.rules.values()
            .filter(|r| matches!(r.detector, DetectorType::Regex { .. }))
            .collect()
    }

    /// 获取 Config 规则
    pub fn config_rules(&self) -> Vec<&RuleDefinition> {
        self.rules.values()
            .filter(|r| matches!(r.detector, DetectorType::Config { .. }))
            .collect()
    }

    /// 规则数量统计
    pub fn stats(&self) -> RegistryStats {
        let total = self.rules.len();
        let p0_count = self.rules.values().filter(|r| r.severity == Severity::P0).count();
        let p1_count = self.rules.values().filter(|r| r.severity == Severity::P1).count();
        let ast_count = self.ast_rules().len();
        let regex_count = self.regex_rules().len();
        let config_count = self.config_rules().len();

        RegistryStats {
            total,
            p0_count,
            p1_count,
            ast_count,
            regex_count,
            config_count,
        }
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 注册表统计信息
#[derive(Debug, Serialize)]
pub struct RegistryStats {
    pub total: usize,
    pub p0_count: usize,
    pub p1_count: usize,
    pub ast_count: usize,
    pub regex_count: usize,
    pub config_count: usize,
}

/// 全局规则注册表 (延迟初始化)
pub static REGISTRY: Lazy<RuleRegistry> = Lazy::new(RuleRegistry::new);

/// 获取全局规则注册表
pub fn registry() -> &'static RuleRegistry {
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = RuleRegistry::new();
        let stats = registry.stats();

        assert!(stats.total > 0, "Should have rules registered");
        assert!(stats.ast_count > 0, "Should have AST rules");
    }

    #[test]
    fn test_get_rule() {
        let registry = RuleRegistry::new();

        let n_plus_one = registry.get("N_PLUS_ONE");
        assert!(n_plus_one.is_some(), "N_PLUS_ONE rule should exist");

        let rule = n_plus_one.unwrap();
        assert_eq!(rule.severity, Severity::P0);
        assert_eq!(rule.category, Category::Performance);
    }
}
