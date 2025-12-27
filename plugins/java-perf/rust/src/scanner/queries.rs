//! 外部化 Query 加载模块
//! 
//! v9.4: 使用 include_str! 在编译时加载 Tree-sitter Query 文件
//! 保持"单二进制文件"优势，同时允许在 .scm 文件中编辑和维护 Query

// ============================================================================
// N+1 问题检测
// ============================================================================

/// N+1 检测 Query (循环内的方法调用)
pub const N_PLUS_ONE: &str = include_str!("../../resources/queries/n_plus_one.scm");

// ============================================================================
// SQL 问题检测
// ============================================================================

/// SQL 问题检测 Query (SELECT *, LIKE 前导通配符)
pub const SQL_ISSUES: &str = include_str!("../../resources/queries/sql_issues.scm");

// ============================================================================
// 并发问题检测
// ============================================================================

/// 并发问题检测 Query (synchronized, 锁泄漏, ThreadLocal)
pub const CONCURRENCY: &str = include_str!("../../resources/queries/concurrency.scm");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_queries_load() {
        // 确保所有 Query 都能正确加载
        assert!(!N_PLUS_ONE.is_empty());
        assert!(!SQL_ISSUES.is_empty());
        assert!(!CONCURRENCY.is_empty());
        
        // 验证包含基本结构
        assert!(N_PLUS_ONE.contains("for_statement"));
        assert!(SQL_ISSUES.contains("string_literal"));
        assert!(CONCURRENCY.contains("synchronized"));
    }
}
