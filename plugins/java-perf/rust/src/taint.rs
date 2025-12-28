// ============================================================================
// 污点分析模块 - 跨文件调用链追踪
// ============================================================================
//
// 🚧 状态: 实验性 (Experimental) - v9.1
//
// 此模块实现了完整的调用图 (Call Graph) 分析能力，用于追踪：
// - Controller -> Service -> Repository 调用链
// - N+1 问题的跨文件传播路径
//
// ## 当前状态:
// - CallGraph 数据结构已完成
// - trace_to_layer() 追踪算法已完成
// - detect_n_plus_one_chains() 检测逻辑已完成
// - **待集成**: 需要在 Phase 1 提取 MethodInvocation 信息
//
// ## 集成计划:
// 1. 在 tree_sitter_java.rs 中添加 extract_call_sites() 方法
// 2. 在 ast_engine.rs Phase 1 中构建 CallGraph
// 3. 在 N+1 检测时使用 trace_to_layer() 验证调用链
// 4. 为 Claude 提供更精确的 "Sniper" 导航能力
//
// ============================================================================

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::symbol_table::{ImportIndex, SymbolTable};

/// 方法签名
/// 
/// v9.8: Updated to use FQN (Fully Qualified Name) for class identification.
/// The class_fqn field contains the full package path (e.g., "com.example.service.UserService")
/// to enable accurate cross-package call chain tracing.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MethodSig {
    /// Fully Qualified Name of the class (e.g., "com.example.service.UserService")
    /// For unresolved types, this may be just the simple name with a marker
    pub class_fqn: String,
    /// Method name (e.g., "getUser")
    pub name: String,
}

impl MethodSig {
    /// Create a new MethodSig with a simple class name (legacy compatibility)
    /// 
    /// Note: Prefer `new_fqn()` when the FQN is known for accurate cross-package analysis.
    pub fn new(class: &str, name: &str) -> Self {
        Self {
            class_fqn: class.to_string(),
            name: name.to_string(),
        }
    }

    /// Create a new MethodSig with a fully qualified class name
    /// 
    /// # Arguments
    /// * `class_fqn` - The fully qualified class name (e.g., "com.example.service.UserService")
    /// * `name` - The method name
    pub fn new_fqn(class_fqn: &str, name: &str) -> Self {
        Self {
            class_fqn: class_fqn.to_string(),
            name: name.to_string(),
        }
    }
    
    /// Get the full method signature (class_fqn.method_name)
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.class_fqn, self.name)
    }

    /// Check if this MethodSig has a valid FQN (contains at least one dot separator)
    /// 
    /// Returns true if the class_fqn appears to be a fully qualified name,
    /// false if it's just a simple class name or marked as unresolved.
    pub fn has_valid_fqn(&self) -> bool {
        self.class_fqn.contains('.') && !self.class_fqn.starts_with("UNRESOLVED:")
    }

    /// Check if this MethodSig is marked as unresolved
    pub fn is_unresolved(&self) -> bool {
        self.class_fqn.starts_with("UNRESOLVED:")
    }

    /// Get the simple class name (last component of FQN)
    pub fn simple_class_name(&self) -> &str {
        if self.class_fqn.starts_with("UNRESOLVED:") {
            &self.class_fqn["UNRESOLVED:".len()..]
        } else {
            self.class_fqn.rsplit('.').next().unwrap_or(&self.class_fqn)
        }
    }

    /// Resolve a simple class name to FQN using ImportIndex and SymbolTable
    /// 
    /// This method attempts to resolve a simple class name (e.g., "UserRepository") to its
    /// fully qualified name (e.g., "com.example.repo.UserRepository") using the provided
    /// ImportIndex for import resolution and SymbolTable for known class lookup.
    /// 
    /// If resolution fails, the class name is marked as "UNRESOLVED:{simple_name}" to indicate
    /// that heuristic detection should be used with reduced confidence.
    /// 
    /// # Arguments
    /// * `simple_class` - The simple class name to resolve (e.g., "UserRepository")
    /// * `method_name` - The method name
    /// * `import_index` - The ImportIndex for the file where the call occurs
    /// * `symbol_table` - The global SymbolTable containing all known classes
    /// 
    /// # Returns
    /// A MethodSig with either:
    /// - A resolved FQN (e.g., "com.example.repo.UserRepository")
    /// - An unresolved marker (e.g., "UNRESOLVED:UserRepository") if resolution fails
    /// 
    /// # Example
    /// ```ignore
    /// let method_sig = MethodSig::resolve("UserRepository", "findById", &import_index, &symbol_table);
    /// if method_sig.has_valid_fqn() {
    ///     // FQN was resolved successfully
    /// } else if method_sig.is_unresolved() {
    ///     // Use heuristic detection with reduced confidence
    /// }
    /// ```
    pub fn resolve(
        simple_class: &str,
        method_name: &str,
        import_index: &ImportIndex,
        symbol_table: &SymbolTable,
    ) -> Self {
        // If the class name already looks like an FQN (contains a dot), use it directly
        if simple_class.contains('.') {
            return Self::new_fqn(simple_class, method_name);
        }

        // Build known_classes map from SymbolTable for ImportIndex resolution
        let known_classes: HashMap<String, String> = symbol_table.classes
            .iter()
            .map(|(fqn, info)| (fqn.clone(), info.name.clone()))
            .collect();

        // Try to resolve using ImportIndex
        if let Some(fqn) = import_index.resolve(simple_class, &known_classes) {
            return Self::new_fqn(&fqn, method_name);
        }

        // Resolution failed - mark as unresolved for heuristic fallback
        Self {
            class_fqn: format!("UNRESOLVED:{}", simple_class),
            name: method_name.to_string(),
        }
    }
}

/// 调用点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSite {
    pub file: PathBuf,
    pub line: usize,
    pub callee: MethodSig,
    pub caller: MethodSig,
}

/// 调用图 - 用于追踪 Controller -> Service -> DAO 链
#[derive(Debug, Default)]
pub struct CallGraph {
    /// 方法签名 -> 该方法调用的其他方法
    pub outgoing: HashMap<MethodSig, Vec<CallSite>>,
    /// 方法签名 -> 调用该方法的其他方法  
    pub incoming: HashMap<MethodSig, Vec<CallSite>>,
    /// 类名 (FQN preferred) -> 文件路径
    pub class_index: HashMap<String, PathBuf>,
    /// Class FQN -> Layer type (Controller/Service/Repository)
    /// 
    /// For accurate cross-package tracing, classes should be registered with their
    /// fully qualified names (e.g., "com.example.service.UserService").
    /// The trace_to_layer() method will first try FQN lookup, then fall back to simple name.
    pub class_layers: HashMap<String, LayerType>,
}

/// 代码层级类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Controller,
    Service,
    Repository,
    Unknown,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// 合并另一个 CallGraph (用于 Rayon 并行 reduce) - v9.4
    pub fn merge(&mut self, other: Self) {
        // 合并 outgoing 边
        for (method, calls) in other.outgoing {
            self.outgoing.entry(method).or_default().extend(calls);
        }
        // 合并 incoming 边
        for (method, calls) in other.incoming {
            self.incoming.entry(method).or_default().extend(calls);
        }
        // 合并类索引
        self.class_index.extend(other.class_index);
        self.class_layers.extend(other.class_layers);
    }
    
    /// 添加调用关系
    pub fn add_call(&mut self, caller: MethodSig, callee: MethodSig, file: PathBuf, line: usize) {
        let call_site = CallSite {
            file: file.clone(),
            line,
            callee: callee.clone(),
            caller: caller.clone(),
        };
        
        // 添加出边
        self.outgoing
            .entry(caller.clone())
            .or_default()
            .push(call_site.clone());
        
        // 添加入边
        self.incoming
            .entry(callee)
            .or_default()
            .push(call_site);
    }
    
    /// Register class information with FQN as the primary key
    /// 
    /// # Arguments
    /// * `class_fqn` - The fully qualified class name (e.g., "com.example.service.UserService")
    ///   For backward compatibility, simple names are also accepted but FQN is preferred
    ///   for accurate cross-package call chain tracing.
    /// * `file` - The file path where the class is defined
    /// * `layer` - The architectural layer type (Controller, Service, Repository, Unknown)
    /// 
    /// # Note
    /// For accurate cross-package tracing (Property 11), always use FQN when available.
    /// The trace_to_layer() method will first try FQN lookup, then fall back to simple name.
    pub fn register_class(&mut self, class_fqn: &str, file: PathBuf, layer: LayerType) {
        self.class_index.insert(class_fqn.to_string(), file);
        self.class_layers.insert(class_fqn.to_string(), layer);
    }
    
    /// Trace from a method to a target architectural layer
    /// 
    /// This method performs a depth-first search through the call graph to find all paths
    /// from the starting method to any method in the target layer (e.g., Repository).
    /// 
    /// # Arguments
    /// * `start` - The starting method signature (should use FQN for accurate cross-package tracing)
    /// * `target_layer` - The target architectural layer to trace to
    /// * `max_depth` - Maximum depth to search (prevents infinite loops)
    /// 
    /// # Returns
    /// A vector of paths, where each path is a vector of MethodSig from start to target.
    /// 
    /// # Cross-Package Tracing (Property 11)
    /// This method supports cross-package call chain tracing by:
    /// 1. First looking up the class layer using the full FQN (class_fqn field)
    /// 2. Falling back to simple class name lookup for backward compatibility
    /// 
    /// For accurate cross-package tracing, ensure that:
    /// - Classes are registered with their FQN via register_class()
    /// - MethodSig instances use FQN in the class_fqn field
    pub fn trace_to_layer(&self, start: &MethodSig, target_layer: LayerType, max_depth: usize) -> Vec<Vec<MethodSig>> {
        let mut paths = Vec::new();
        let mut current_path = vec![start.clone()];
        let mut visited = std::collections::HashSet::new();
        
        self.dfs_trace(start, target_layer, max_depth, &mut current_path, &mut visited, &mut paths);
        
        paths
    }
    
    fn dfs_trace(
        &self,
        current: &MethodSig,
        target_layer: LayerType,
        remaining_depth: usize,
        path: &mut Vec<MethodSig>,
        visited: &mut std::collections::HashSet<MethodSig>,
        result: &mut Vec<Vec<MethodSig>>,
    ) {
        if remaining_depth == 0 {
            return;
        }
        
        // 检查当前方法是否在目标层
        // Try FQN first, then fall back to simple class name for backward compatibility
        let layer = self.class_layers.get(&current.class_fqn)
            .or_else(|| self.class_layers.get(current.simple_class_name()));
        
        if let Some(layer) = layer {
            if *layer == target_layer && path.len() > 1 {
                result.push(path.clone());
                return;
            }
        }
        
        // 继续 DFS
        if let Some(callees) = self.outgoing.get(current) {
            for call_site in callees {
                if !visited.contains(&call_site.callee) {
                    visited.insert(call_site.callee.clone());
                    path.push(call_site.callee.clone());
                    
                    self.dfs_trace(&call_site.callee, target_layer, remaining_depth - 1, path, visited, result);
                    
                    path.pop();
                    visited.remove(&call_site.callee);
                }
            }
        }
    }
    
    /// 检测 N+1 问题：在循环内调用的方法最终是否到达 Repository
    pub fn detect_n_plus_one_chains(&self) -> Vec<CallChainReport> {
        let mut reports = Vec::new();
        
        // 查找所有 Repository 方法
        for (method, incoming_calls) in &self.incoming {
            // Try FQN first, then fall back to simple class name for backward compatibility
            let layer = self.class_layers.get(&method.class_fqn)
                .or_else(|| self.class_layers.get(method.simple_class_name()));
            
            if let Some(layer) = layer {
                if *layer == LayerType::Repository {
                    // 对每个调用点，追踪回到 Controller
                    for call_site in incoming_calls {
                        let paths = self.trace_to_layer(&call_site.caller, LayerType::Controller, 5);
                        if !paths.is_empty() {
                            reports.push(CallChainReport {
                                dao_method: method.clone(),
                                call_site: call_site.clone(),
                                controller_paths: paths,
                            });
                        }
                    }
                }
            }
        }
        
        reports
    }
}

/// 调用链报告
#[derive(Debug, Serialize)]
pub struct CallChainReport {
    pub dao_method: MethodSig,
    pub call_site: CallSite,
    pub controller_paths: Vec<Vec<MethodSig>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    #[test]
    fn test_call_graph_basic() {
        let mut graph = CallGraph::new();
        
        // 注册类
        graph.register_class("UserController", PathBuf::from("UserController.java"), LayerType::Controller);
        graph.register_class("UserService", PathBuf::from("UserService.java"), LayerType::Service);
        graph.register_class("UserRepository", PathBuf::from("UserRepository.java"), LayerType::Repository);
        
        // Controller -> Service
        graph.add_call(
            MethodSig::new("UserController", "getUsers"),
            MethodSig::new("UserService", "findAll"),
            PathBuf::from("UserController.java"),
            10,
        );
        
        // Service -> Repository
        graph.add_call(
            MethodSig::new("UserService", "findAll"),
            MethodSig::new("UserRepository", "findById"),
            PathBuf::from("UserService.java"),
            20,
        );
        
        // 追踪 Controller -> Repository
        let paths = graph.trace_to_layer(
            &MethodSig::new("UserController", "getUsers"),
            LayerType::Repository,
            5,
        );
        
        assert!(!paths.is_empty(), "Should find path from Controller to Repository");
        assert_eq!(paths[0].len(), 3); // Controller -> Service -> Repository
    }

    // ========================================================================
    // MethodSig Unit Tests
    // ========================================================================

    #[test]
    fn test_method_sig_new_fqn() {
        let sig = MethodSig::new_fqn("com.example.service.UserService", "findById");
        assert_eq!(sig.class_fqn, "com.example.service.UserService");
        assert_eq!(sig.name, "findById");
        assert!(sig.has_valid_fqn());
        assert!(!sig.is_unresolved());
        assert_eq!(sig.simple_class_name(), "UserService");
    }

    #[test]
    fn test_method_sig_simple_name() {
        let sig = MethodSig::new("UserService", "findById");
        assert_eq!(sig.class_fqn, "UserService");
        assert!(!sig.has_valid_fqn()); // No dot, so not a valid FQN
        assert!(!sig.is_unresolved());
        assert_eq!(sig.simple_class_name(), "UserService");
    }

    #[test]
    fn test_method_sig_unresolved() {
        let sig = MethodSig {
            class_fqn: "UNRESOLVED:UserService".to_string(),
            name: "findById".to_string(),
        };
        assert!(!sig.has_valid_fqn());
        assert!(sig.is_unresolved());
        assert_eq!(sig.simple_class_name(), "UserService");
    }

    #[test]
    fn test_method_sig_resolve_with_fqn_input() {
        // When input already contains a dot, it should be used directly
        let import_index = ImportIndex::default();
        let symbol_table = SymbolTable::new();
        
        let sig = MethodSig::resolve("com.example.UserService", "findById", &import_index, &symbol_table);
        assert_eq!(sig.class_fqn, "com.example.UserService");
        assert!(sig.has_valid_fqn());
    }

    #[test]
    fn test_method_sig_resolve_unresolvable() {
        // When resolution fails, it should be marked as unresolved
        let import_index = ImportIndex::default();
        let symbol_table = SymbolTable::new();
        
        let sig = MethodSig::resolve("UnknownClass", "someMethod", &import_index, &symbol_table);
        assert!(sig.is_unresolved());
        assert_eq!(sig.simple_class_name(), "UnknownClass");
    }

    // ========================================================================
    // Property Tests
    // ========================================================================

    /// Strategy to generate valid Java package names
    fn java_package_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-z][a-z0-9]{0,7}", 1..=4)
            .prop_map(|parts| parts.join("."))
    }

    /// Strategy to generate valid Java class names (PascalCase)
    fn java_class_name_strategy() -> impl Strategy<Value = String> {
        "[A-Z][a-zA-Z0-9]{0,15}".prop_filter("Must be valid class name", |s| {
            !s.is_empty() && s.chars().next().unwrap().is_uppercase()
        })
    }

    /// Strategy to generate valid Java method names (camelCase)
    fn java_method_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9]{0,15}".prop_filter("Must be valid method name", |s| {
            !s.is_empty() && s.chars().next().unwrap().is_lowercase()
        })
    }

    proptest! {
        /// **Feature: java-perf-semantic-analysis, Property 4: CallGraph FQN Format**
        /// 
        /// *For any* MethodSig in the CallGraph, the class field SHALL contain a valid FQN 
        /// (containing at least one dot separator) or be marked as unresolved.
        /// 
        /// **Validates: Requirements 1.4**
        #[test]
        fn prop_callgraph_fqn_format(
            pkg in java_package_strategy(),
            class_name in java_class_name_strategy(),
            method_name in java_method_name_strategy(),
        ) {
            // Create a MethodSig using new_fqn (the recommended way)
            let fqn = format!("{}.{}", pkg, class_name);
            let sig = MethodSig::new_fqn(&fqn, &method_name);

            // Property 1: FQN should contain at least one dot
            prop_assert!(
                sig.class_fqn.contains('.'),
                "FQN '{}' should contain at least one dot separator",
                sig.class_fqn
            );

            // Property 2: has_valid_fqn() should return true for properly constructed FQNs
            prop_assert!(
                sig.has_valid_fqn(),
                "MethodSig with FQN '{}' should have valid FQN",
                sig.class_fqn
            );

            // Property 3: is_unresolved() should return false for properly constructed FQNs
            prop_assert!(
                !sig.is_unresolved(),
                "MethodSig with FQN '{}' should not be marked as unresolved",
                sig.class_fqn
            );

            // Property 4: simple_class_name() should return the last component
            prop_assert_eq!(
                sig.simple_class_name(),
                class_name.as_str(),
                "simple_class_name() should return '{}' for FQN '{}'",
                class_name, sig.class_fqn
            );

            // Property 5: full_name() should combine class_fqn and method name
            let expected_full_name = format!("{}.{}", fqn, method_name);
            prop_assert_eq!(
                sig.full_name(),
                expected_full_name.clone(),
                "full_name() should return '{}'",
                &expected_full_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 4 (continued): Unresolved marking**
        /// 
        /// *For any* simple class name that cannot be resolved, the MethodSig SHALL be marked
        /// as unresolved with the "UNRESOLVED:" prefix.
        /// 
        /// **Validates: Requirements 1.4, 4.4**
        #[test]
        fn prop_callgraph_unresolved_format(
            class_name in java_class_name_strategy(),
            method_name in java_method_name_strategy(),
        ) {
            // Create an empty ImportIndex and SymbolTable (no resolution possible)
            let import_index = ImportIndex::default();
            let symbol_table = SymbolTable::new();

            // Resolve should fail and mark as unresolved
            let sig = MethodSig::resolve(&class_name, &method_name, &import_index, &symbol_table);

            // Property 1: Unresolved MethodSig should be marked with UNRESOLVED: prefix
            prop_assert!(
                sig.is_unresolved(),
                "MethodSig for unresolvable class '{}' should be marked as unresolved",
                &class_name
            );

            // Property 2: has_valid_fqn() should return false for unresolved
            prop_assert!(
                !sig.has_valid_fqn(),
                "Unresolved MethodSig should not have valid FQN"
            );

            // Property 3: simple_class_name() should still return the original class name
            prop_assert_eq!(
                sig.simple_class_name(),
                class_name.as_str(),
                "simple_class_name() should return '{}' for unresolved MethodSig",
                &class_name
            );

            // Property 4: class_fqn should have the UNRESOLVED: prefix
            let expected_prefix = format!("UNRESOLVED:{}", class_name);
            prop_assert_eq!(
                &sig.class_fqn,
                &expected_prefix,
                "class_fqn should be '{}' for unresolved MethodSig",
                &expected_prefix
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 4 (continued): Resolution with FQN input**
        /// 
        /// *For any* class name that already contains a dot (looks like FQN), the resolve()
        /// method SHALL use it directly without modification.
        /// 
        /// **Validates: Requirements 1.4**
        #[test]
        fn prop_callgraph_fqn_passthrough(
            pkg in java_package_strategy(),
            class_name in java_class_name_strategy(),
            method_name in java_method_name_strategy(),
        ) {
            let fqn = format!("{}.{}", pkg, class_name);
            let import_index = ImportIndex::default();
            let symbol_table = SymbolTable::new();

            // When input already looks like FQN, it should pass through
            let sig = MethodSig::resolve(&fqn, &method_name, &import_index, &symbol_table);

            // Property 1: FQN should be preserved
            prop_assert_eq!(
                &sig.class_fqn,
                &fqn,
                "FQN '{}' should be preserved when passed to resolve()",
                &fqn
            );

            // Property 2: Should have valid FQN
            prop_assert!(
                sig.has_valid_fqn(),
                "MethodSig with FQN '{}' should have valid FQN",
                &sig.class_fqn
            );

            // Property 3: Should not be marked as unresolved
            prop_assert!(
                !sig.is_unresolved(),
                "MethodSig with FQN '{}' should not be marked as unresolved",
                &sig.class_fqn
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 10: Field Type Resolution to CallGraph**
        /// 
        /// *For any* method call on a field where the field's type is registered in SymbolTable,
        /// the resulting CallGraph edge SHALL use the resolved FQN.
        /// 
        /// **Validates: Requirements 4.1, 4.2**
        #[test]
        fn prop_field_type_resolution_to_callgraph(
            // Field type class info
            field_type_pkg in java_package_strategy(),
            field_type_class in java_class_name_strategy(),
            // Caller class info
            caller_pkg in java_package_strategy(),
            _caller_class in java_class_name_strategy(),
            // Field and method names
            _field_name in "[a-z][a-zA-Z0-9]{0,10}",
            method_name in java_method_name_strategy(),
        ) {
            use crate::symbol_table::TypeInfo;
            
            // Ensure packages are different to make the test meaningful
            prop_assume!(field_type_pkg != caller_pkg);
            
            // Create the field type FQN
            let field_type_fqn = format!("{}.{}", field_type_pkg, field_type_class);
            
            // Create a SymbolTable with the field type registered
            let mut symbol_table = SymbolTable::new();
            let field_type_info = TypeInfo::new_with_package(
                &field_type_class,
                Some(&field_type_pkg),
                std::path::PathBuf::from("FieldType.java"),
                1,
            );
            symbol_table.register_class_fqn(field_type_info);
            
            // Create an ImportIndex with an explicit import for the field type
            let import_index = ImportIndex::from_imports(
                vec![field_type_fqn.clone()],
                Some(caller_pkg.clone()),
            );
            
            // Resolve the field type (simulating a method call on a field)
            // In real code, this would be: fieldName.methodName()
            // The receiver would be the field type class name
            let sig = MethodSig::resolve(&field_type_class, &method_name, &import_index, &symbol_table);
            
            // Property 1: The resolved MethodSig should have the correct FQN
            prop_assert_eq!(
                &sig.class_fqn,
                &field_type_fqn,
                "Field type '{}' should resolve to FQN '{}', got '{}'",
                &field_type_class, &field_type_fqn, &sig.class_fqn
            );
            
            // Property 2: The MethodSig should have a valid FQN
            prop_assert!(
                sig.has_valid_fqn(),
                "Resolved MethodSig should have valid FQN"
            );
            
            // Property 3: The MethodSig should not be marked as unresolved
            prop_assert!(
                !sig.is_unresolved(),
                "Resolved MethodSig should not be marked as unresolved"
            );
            
            // Property 4: The simple class name should match the original
            prop_assert_eq!(
                sig.simple_class_name(),
                field_type_class.as_str(),
                "simple_class_name() should return the original class name"
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 10 (continued): Wildcard import resolution**
        /// 
        /// *For any* field type that is imported via wildcard import and registered in SymbolTable,
        /// the resulting CallGraph edge SHALL use the resolved FQN.
        /// 
        /// **Validates: Requirements 4.1, 4.2**
        #[test]
        fn prop_field_type_resolution_wildcard(
            // Field type class info
            field_type_pkg in java_package_strategy(),
            field_type_class in java_class_name_strategy(),
            // Caller class info
            caller_pkg in java_package_strategy(),
            // Method name
            method_name in java_method_name_strategy(),
        ) {
            use crate::symbol_table::TypeInfo;
            
            // Ensure packages are different
            prop_assume!(field_type_pkg != caller_pkg);
            
            // Create the field type FQN
            let field_type_fqn = format!("{}.{}", field_type_pkg, field_type_class);
            
            // Create a SymbolTable with the field type registered
            let mut symbol_table = SymbolTable::new();
            let field_type_info = TypeInfo::new_with_package(
                &field_type_class,
                Some(&field_type_pkg),
                std::path::PathBuf::from("FieldType.java"),
                1,
            );
            symbol_table.register_class_fqn(field_type_info);
            
            // Create an ImportIndex with a wildcard import for the field type's package
            let wildcard_import = format!("{}.*", field_type_pkg);
            let import_index = ImportIndex::from_imports(
                vec![wildcard_import],
                Some(caller_pkg.clone()),
            );
            
            // Build known_classes from SymbolTable for wildcard resolution
            let known_classes: std::collections::HashMap<String, String> = symbol_table.classes
                .iter()
                .map(|(fqn, info)| (fqn.clone(), info.name.clone()))
                .collect();
            
            // Resolve using ImportIndex directly (to test wildcard resolution)
            let resolved_fqn = import_index.resolve(&field_type_class, &known_classes);
            
            // Property 1: Wildcard import should resolve to the correct FQN
            prop_assert_eq!(
                resolved_fqn.as_ref(),
                Some(&field_type_fqn),
                "Wildcard import should resolve '{}' to FQN '{}'",
                &field_type_class, &field_type_fqn
            );
            
            // Now test through MethodSig::resolve
            let sig = MethodSig::resolve(&field_type_class, &method_name, &import_index, &symbol_table);
            
            // Property 2: MethodSig should have the resolved FQN
            prop_assert_eq!(
                &sig.class_fqn,
                &field_type_fqn,
                "MethodSig should have resolved FQN"
            );
            
            // Property 3: Should have valid FQN
            prop_assert!(
                sig.has_valid_fqn(),
                "Resolved MethodSig should have valid FQN"
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 11: Cross-Package Call Chain Tracing**
        /// 
        /// *For any* call chain from a Controller class to a Repository class through Service 
        /// classes in different packages, trace_to_layer() SHALL find the complete path.
        /// 
        /// **Validates: Requirements 4.3**
        #[test]
        fn prop_cross_package_call_chain_tracing(
            // Controller package and class
            controller_pkg in java_package_strategy(),
            controller_class in java_class_name_strategy(),
            controller_method in java_method_name_strategy(),
            // Service package and class (different package)
            service_pkg in java_package_strategy(),
            service_class in java_class_name_strategy(),
            service_method in java_method_name_strategy(),
            // Repository package and class (different package)
            repo_pkg in java_package_strategy(),
            repo_class in java_class_name_strategy(),
            repo_method in java_method_name_strategy(),
        ) {
            // Ensure all packages are different to test cross-package tracing
            prop_assume!(controller_pkg != service_pkg);
            prop_assume!(service_pkg != repo_pkg);
            prop_assume!(controller_pkg != repo_pkg);
            
            // Ensure class names are different to avoid confusion
            prop_assume!(controller_class != service_class);
            prop_assume!(service_class != repo_class);
            prop_assume!(controller_class != repo_class);

            // Build FQNs for each class
            let controller_fqn = format!("{}.{}", controller_pkg, controller_class);
            let service_fqn = format!("{}.{}", service_pkg, service_class);
            let repo_fqn = format!("{}.{}", repo_pkg, repo_class);

            // Create CallGraph and register classes with FQNs
            let mut graph = CallGraph::new();
            
            // Register classes with their FQNs
            graph.register_class(&controller_fqn, PathBuf::from("Controller.java"), LayerType::Controller);
            graph.register_class(&service_fqn, PathBuf::from("Service.java"), LayerType::Service);
            graph.register_class(&repo_fqn, PathBuf::from("Repository.java"), LayerType::Repository);

            // Create method signatures using FQNs
            let controller_sig = MethodSig::new_fqn(&controller_fqn, &controller_method);
            let service_sig = MethodSig::new_fqn(&service_fqn, &service_method);
            let repo_sig = MethodSig::new_fqn(&repo_fqn, &repo_method);

            // Add call chain: Controller -> Service -> Repository
            graph.add_call(
                controller_sig.clone(),
                service_sig.clone(),
                PathBuf::from("Controller.java"),
                10,
            );
            graph.add_call(
                service_sig.clone(),
                repo_sig.clone(),
                PathBuf::from("Service.java"),
                20,
            );

            // Property 1: trace_to_layer from Controller should find path to Repository
            let paths = graph.trace_to_layer(&controller_sig, LayerType::Repository, 5);
            
            prop_assert!(
                !paths.is_empty(),
                "Should find path from Controller ({}) to Repository ({}) through Service ({})",
                &controller_fqn, &repo_fqn, &service_fqn
            );

            // Property 2: The path should have exactly 3 nodes (Controller -> Service -> Repository)
            prop_assert_eq!(
                paths[0].len(),
                3,
                "Path should have 3 nodes: Controller -> Service -> Repository. Got {} nodes",
                paths[0].len()
            );

            // Property 3: First node should be the Controller method
            prop_assert_eq!(
                &paths[0][0].class_fqn,
                &controller_fqn,
                "First node should be Controller FQN '{}', got '{}'",
                &controller_fqn, &paths[0][0].class_fqn
            );

            // Property 4: Second node should be the Service method
            prop_assert_eq!(
                &paths[0][1].class_fqn,
                &service_fqn,
                "Second node should be Service FQN '{}', got '{}'",
                &service_fqn, &paths[0][1].class_fqn
            );

            // Property 5: Third node should be the Repository method
            prop_assert_eq!(
                &paths[0][2].class_fqn,
                &repo_fqn,
                "Third node should be Repository FQN '{}', got '{}'",
                &repo_fqn, &paths[0][2].class_fqn
            );

            // Property 6: All FQNs in the path should be valid (contain dots)
            for (i, method_sig) in paths[0].iter().enumerate() {
                prop_assert!(
                    method_sig.has_valid_fqn(),
                    "Node {} in path should have valid FQN, got '{}'",
                    i, &method_sig.class_fqn
                );
            }

            // Property 7: trace_to_layer from Service should find path to Repository
            let service_paths = graph.trace_to_layer(&service_sig, LayerType::Repository, 5);
            prop_assert!(
                !service_paths.is_empty(),
                "Should find path from Service ({}) to Repository ({})",
                &service_fqn, &repo_fqn
            );
            prop_assert_eq!(
                service_paths[0].len(),
                2,
                "Path from Service to Repository should have 2 nodes"
            );

            // Property 8: trace_to_layer from Controller to Controller should return empty
            // (no path to itself at a different layer)
            let no_paths = graph.trace_to_layer(&controller_sig, LayerType::Controller, 5);
            prop_assert!(
                no_paths.is_empty(),
                "Should not find path from Controller to Controller layer"
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 11 (continued): Multiple paths**
        /// 
        /// *For any* call graph with multiple paths from Controller to Repository,
        /// trace_to_layer() SHALL find all valid paths.
        /// 
        /// **Validates: Requirements 4.3**
        #[test]
        fn prop_cross_package_multiple_paths(
            // Controller
            controller_pkg in java_package_strategy(),
            controller_class in java_class_name_strategy(),
            controller_method in java_method_name_strategy(),
            // Service 1
            service1_pkg in java_package_strategy(),
            service1_class in java_class_name_strategy(),
            service1_method in java_method_name_strategy(),
            // Service 2
            service2_pkg in java_package_strategy(),
            service2_class in java_class_name_strategy(),
            service2_method in java_method_name_strategy(),
            // Repository
            repo_pkg in java_package_strategy(),
            repo_class in java_class_name_strategy(),
            repo_method in java_method_name_strategy(),
        ) {
            // Ensure all packages are different
            prop_assume!(controller_pkg != service1_pkg);
            prop_assume!(controller_pkg != service2_pkg);
            prop_assume!(controller_pkg != repo_pkg);
            prop_assume!(service1_pkg != service2_pkg);
            prop_assume!(service1_pkg != repo_pkg);
            prop_assume!(service2_pkg != repo_pkg);
            
            // Ensure class names are different
            prop_assume!(controller_class != service1_class);
            prop_assume!(controller_class != service2_class);
            prop_assume!(controller_class != repo_class);
            prop_assume!(service1_class != service2_class);
            prop_assume!(service1_class != repo_class);
            prop_assume!(service2_class != repo_class);

            // Build FQNs
            let controller_fqn = format!("{}.{}", controller_pkg, controller_class);
            let service1_fqn = format!("{}.{}", service1_pkg, service1_class);
            let service2_fqn = format!("{}.{}", service2_pkg, service2_class);
            let repo_fqn = format!("{}.{}", repo_pkg, repo_class);

            let mut graph = CallGraph::new();
            
            // Register classes
            graph.register_class(&controller_fqn, PathBuf::from("Controller.java"), LayerType::Controller);
            graph.register_class(&service1_fqn, PathBuf::from("Service1.java"), LayerType::Service);
            graph.register_class(&service2_fqn, PathBuf::from("Service2.java"), LayerType::Service);
            graph.register_class(&repo_fqn, PathBuf::from("Repository.java"), LayerType::Repository);

            // Create method signatures
            let controller_sig = MethodSig::new_fqn(&controller_fqn, &controller_method);
            let service1_sig = MethodSig::new_fqn(&service1_fqn, &service1_method);
            let service2_sig = MethodSig::new_fqn(&service2_fqn, &service2_method);
            let repo_sig = MethodSig::new_fqn(&repo_fqn, &repo_method);

            // Add two paths: Controller -> Service1 -> Repository
            //                Controller -> Service2 -> Repository
            graph.add_call(controller_sig.clone(), service1_sig.clone(), PathBuf::from("Controller.java"), 10);
            graph.add_call(controller_sig.clone(), service2_sig.clone(), PathBuf::from("Controller.java"), 15);
            graph.add_call(service1_sig.clone(), repo_sig.clone(), PathBuf::from("Service1.java"), 20);
            graph.add_call(service2_sig.clone(), repo_sig.clone(), PathBuf::from("Service2.java"), 25);

            // Property 1: Should find both paths
            let paths = graph.trace_to_layer(&controller_sig, LayerType::Repository, 5);
            
            prop_assert!(
                paths.len() >= 2,
                "Should find at least 2 paths from Controller to Repository, found {}",
                paths.len()
            );

            // Property 2: All paths should end at Repository
            for (i, path) in paths.iter().enumerate() {
                let last = path.last().unwrap();
                prop_assert_eq!(
                    &last.class_fqn,
                    &repo_fqn,
                    "Path {} should end at Repository FQN '{}', got '{}'",
                    i, &repo_fqn, &last.class_fqn
                );
            }

            // Property 3: All paths should start at Controller
            for (i, path) in paths.iter().enumerate() {
                let first = path.first().unwrap();
                prop_assert_eq!(
                    &first.class_fqn,
                    &controller_fqn,
                    "Path {} should start at Controller FQN '{}', got '{}'",
                    i, &controller_fqn, &first.class_fqn
                );
            }
        }
    }
}
