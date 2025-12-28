//! 外部化 Query 加载模块
//! 
//! v9.4: 使用 include_str! 在编译时加载 Tree-sitter Query 文件
//! 保持"单二进制文件"优势，同时允许在 .scm 文件中编辑和维护 Query
//!
//! 注意: 当前查询已内联到 tree_sitter_java.rs 的 compile_rules() 中。
//! 这些外部 .scm 文件保留用于未来的查询外部化重构。

// 当前这些查询文件存在但未被使用，因为查询已内联到 compile_rules() 中。
// 保留此模块结构以便将来重构时使用。

#[cfg(test)]
mod tests {
    use std::path::Path;
    
    #[test]
    fn test_query_files_exist() {
        // 验证查询文件存在（即使当前未使用）
        let queries_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/queries");
        assert!(queries_dir.join("n_plus_one.scm").exists(), "n_plus_one.scm should exist");
        assert!(queries_dir.join("sql_issues.scm").exists(), "sql_issues.scm should exist");
        assert!(queries_dir.join("concurrency.scm").exists(), "concurrency.scm should exist");
    }
}
