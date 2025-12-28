// ============================================================================
// 符号表模块 - 轻量级类型追踪
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

// ============================================================================
// ImportIndex - Per-file import resolution index
// ============================================================================

/// Import resolution index for a single Java file
/// 
/// Provides efficient FQN resolution from simple class names using the file's
/// import statements. Resolution follows Java's standard priority:
/// 1. Explicit imports (e.g., `import com.example.UserRepository`)
/// 2. Wildcard imports (e.g., `import com.example.*`)
/// 3. Same-package classes
/// 4. java.lang classes (implicitly imported)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportIndex {
    /// Explicit imports: simple name -> FQN
    /// e.g., "UserRepository" -> "com.example.repo.UserRepository"
    pub explicit: HashMap<String, String>,
    /// Wildcard import packages
    /// e.g., ["com.example.repo", "java.util"]
    pub wildcards: Vec<String>,
    /// Current file's package
    /// e.g., Some("com.example.service")
    pub package: Option<String>,
    /// Classes defined in this file (auto-imported within same package)
    /// e.g., ["UserService", "UserServiceImpl"]
    pub local_classes: Vec<String>,
}

impl ImportIndex {
    /// Build ImportIndex from parsed import statements and package declaration
    /// 
    /// # Arguments
    /// * `imports` - List of import statements (e.g., "com.example.UserRepository", "java.util.*")
    /// * `package` - The package declaration of the current file
    /// 
    /// # Returns
    /// A new ImportIndex with imports categorized as explicit or wildcard
    pub fn from_imports(imports: Vec<String>, package: Option<String>) -> Self {
        let mut explicit = HashMap::new();
        let mut wildcards = Vec::new();

        for import in imports {
            let import = import.trim();
            if import.ends_with(".*") {
                // Wildcard import: extract package name
                let pkg = import.trim_end_matches(".*");
                wildcards.push(pkg.to_string());
            } else if !import.is_empty() {
                // Explicit import: extract simple name as key
                if let Some(simple_name) = import.rsplit('.').next() {
                    explicit.insert(simple_name.to_string(), import.to_string());
                }
            }
        }

        Self {
            explicit,
            wildcards,
            package,
            local_classes: Vec::new(),
        }
    }

    /// Resolve a simple class name to its FQN
    /// 
    /// Resolution priority:
    /// 1. Explicit imports (O(1) lookup)
    /// 2. Wildcard imports (check against known_classes)
    /// 3. Same-package classes
    /// 4. java.lang classes
    /// 
    /// # Arguments
    /// * `simple_name` - The simple class name to resolve (e.g., "UserRepository")
    /// * `known_classes` - Map of FQN -> TypeInfo for all known classes in the project
    /// 
    /// # Returns
    /// The resolved FQN, or None if unresolvable
    pub fn resolve(&self, simple_name: &str, known_classes: &HashMap<String, String>) -> Option<String> {
        // 1. Check explicit imports first (O(1))
        if let Some(fqn) = self.explicit.get(simple_name) {
            return Some(fqn.clone());
        }

        // 2. Check wildcard imports against known classes
        for wildcard_pkg in &self.wildcards {
            let candidate_fqn = format!("{}.{}", wildcard_pkg, simple_name);
            if known_classes.contains_key(&candidate_fqn) {
                return Some(candidate_fqn);
            }
        }

        // 3. Check same-package classes
        if let Some(ref pkg) = self.package {
            // Check local classes defined in this file
            if self.local_classes.contains(&simple_name.to_string()) {
                return Some(format!("{}.{}", pkg, simple_name));
            }
            // Check known classes in same package
            let same_pkg_fqn = format!("{}.{}", pkg, simple_name);
            if known_classes.contains_key(&same_pkg_fqn) {
                return Some(same_pkg_fqn);
            }
        }

        // 4. Check java.lang (implicitly imported)
        let java_lang_fqn = format!("java.lang.{}", simple_name);
        if is_java_lang_class(simple_name) || known_classes.contains_key(&java_lang_fqn) {
            return Some(java_lang_fqn);
        }

        None
    }

    /// Add a local class to the index
    pub fn add_local_class(&mut self, class_name: &str) {
        if !self.local_classes.contains(&class_name.to_string()) {
            self.local_classes.push(class_name.to_string());
        }
    }
}

// ============================================================================
// PackageClassIndex - Global class index for cross-file resolution
// ============================================================================

/// Global index of all classes in the project, organized by package
/// 
/// This structure aggregates class information from all ImportIndex instances
/// to enable cross-file FQN resolution.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct PackageClassIndex {
    /// FQN -> simple name mapping
    pub fqn_to_simple: HashMap<String, String>,
    /// Simple name -> list of FQNs (for classes with same name in different packages)
    pub simple_to_fqns: HashMap<String, Vec<String>>,
}

#[allow(dead_code)]
impl PackageClassIndex {
    /// Build PackageClassIndex from a collection of ImportIndex instances
    /// 
    /// # Arguments
    /// * `import_indices` - Map of file path -> ImportIndex
    /// 
    /// # Returns
    /// A new PackageClassIndex containing all local classes from all files
    pub fn from_import_indices(import_indices: &HashMap<String, ImportIndex>) -> Self {
        let mut fqn_to_simple = HashMap::new();
        let mut simple_to_fqns: HashMap<String, Vec<String>> = HashMap::new();

        for index in import_indices.values() {
            if let Some(ref pkg) = index.package {
                for class_name in &index.local_classes {
                    let fqn = format!("{}.{}", pkg, class_name);
                    fqn_to_simple.insert(fqn.clone(), class_name.clone());
                    simple_to_fqns.entry(class_name.clone()).or_default().push(fqn);
                }
            }
        }

        Self {
            fqn_to_simple,
            simple_to_fqns,
        }
    }

    /// Convert to known_classes format for ImportIndex::resolve
    pub fn to_known_classes(&self) -> HashMap<String, String> {
        self.fqn_to_simple.clone()
    }
}

/// Check if a class name is a common java.lang class
fn is_java_lang_class(name: &str) -> bool {
    matches!(
        name,
        "String" | "Object" | "Integer" | "Long" | "Double" | "Float" 
        | "Boolean" | "Byte" | "Short" | "Character" | "Number"
        | "Class" | "System" | "Thread" | "Runnable" | "Exception"
        | "RuntimeException" | "Error" | "Throwable" | "StringBuilder"
        | "StringBuffer" | "Math" | "Comparable" | "Iterable" | "Enum"
        | "Override" | "Deprecated" | "SuppressWarnings" | "FunctionalInterface"
    )
}

/// 代码层级类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Controller,
    Service,
    Repository,
    Component,
    Unknown,
}

impl LayerType {
    /// 从注解名称推断层级
    pub fn from_annotation(annotation: &str) -> Self {
        match annotation {
            "Controller" | "RestController" => LayerType::Controller,
            "Service" => LayerType::Service,
            "Repository" | "Mapper" => LayerType::Repository,
            "Component" => LayerType::Component,
            _ => LayerType::Unknown,
        }
    }
    
}

/// 类型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,               // "UserRepository"
    pub fqn: String,                // "com.example.repository.UserRepository" (Fully Qualified Name)
    pub package: Option<String>,    // "com.example.repository"
    pub annotations: Vec<String>,   // ["Repository", "Component"]
    pub layer: LayerType,
    pub file: PathBuf,
    pub line: usize,
}

impl TypeInfo {
    /// Create a new TypeInfo with simple name (legacy method for backward compatibility)
    /// 
    /// Note: Prefer `new_with_package` for new code as it properly handles FQN
    #[allow(dead_code)]
    pub fn new(name: &str, file: PathBuf, line: usize) -> Self {
        Self {
            name: name.to_string(),
            fqn: name.to_string(),
            package: None,
            annotations: Vec::new(),
            layer: LayerType::Unknown,
            file,
            line,
        }
    }

    /// Create a new TypeInfo with package information to build proper FQN
    pub fn new_with_package(name: &str, package: Option<&str>, file: PathBuf, line: usize) -> Self {
        let fqn = match package {
            Some(pkg) if !pkg.is_empty() => format!("{}.{}", pkg, name),
            _ => name.to_string(),
        };
        Self {
            name: name.to_string(),
            fqn,
            package: package.map(|s| s.to_string()),
            annotations: Vec::new(),
            layer: LayerType::Unknown,
            file,
            line,
        }
    }
    
    /// 添加注解并更新层级
    pub fn add_annotation(&mut self, annotation: &str) {
        self.annotations.push(annotation.to_string());
        // 更新层级（取优先级最高的）
        let new_layer = LayerType::from_annotation(annotation);
        if new_layer != LayerType::Unknown {
            self.layer = new_layer;
        }
    }
    
    /// 判断是否是 DAO 类型
    pub fn is_dao(&self) -> bool {
        self.layer == LayerType::Repository
            || self.annotations.iter().any(|a| {
                a == "Repository" || a == "Mapper" || a.ends_with("Repository") || a.ends_with("Dao")
            })
            || self.name.ends_with("Repository")
            || self.name.ends_with("Dao")
            || self.name.ends_with("Mapper")
    }
}

/// 变量绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarBinding {
    pub name: String,           // "userRepository"
    pub type_name: String,      // "UserRepository"
    pub is_field: bool,         // 是否是字段（而非局部变量）
    pub annotations: Vec<String>, // 字段上的注解，如 ["Autowired"]
}

impl VarBinding {
    pub fn new(name: &str, type_name: &str, is_field: bool) -> Self {
        Self {
            name: name.to_string(),
            type_name: type_name.to_string(),
            is_field,
            annotations: Vec::new(),
        }
    }
}

/// 方法参数信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
}

/// 方法信息 (v9.2: 增强版，支持参数签名)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub class: String,
    pub return_type: Option<String>,
    pub params: Vec<ParamInfo>,  // v9.2: 参数列表
    pub annotations: Vec<String>,
    pub line: usize,
}

// Test-only builder methods
#[cfg(test)]
impl MethodInfo {
    pub fn new(name: &str, class: &str, line: usize) -> Self {
        Self {
            name: name.to_string(),
            class: class.to_string(),
            return_type: None,
            params: Vec::new(),
            annotations: Vec::new(),
            line,
        }
    }

    /// 生成方法签名 (用于区分重载)
    pub fn signature(&self) -> String {
        let param_types: Vec<&str> = self.params.iter()
            .map(|p| p.type_name.as_str())
            .collect();
        format!("{}({})", self.name, param_types.join(","))
    }

    /// 添加参数
    pub fn add_param(&mut self, name: &str, type_name: &str) {
        self.params.push(ParamInfo {
            name: name.to_string(),
            type_name: type_name.to_string(),
        });
    }
}

/// 符号表 - 跟踪类型和变量 (v9.2: 支持方法重载)
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// 类名 -> 类型信息 (keyed by FQN for uniqueness)
    pub classes: HashMap<String, TypeInfo>,
    /// Simple name -> Vec<FQN> for reverse lookup (handles same-name classes in different packages)
    pub simple_name_index: HashMap<String, Vec<String>>,
    /// (类名, 字段名) -> 变量绑定
    pub fields: HashMap<(String, String), VarBinding>,
    /// (类名, 方法签名) -> 方法信息
    /// 注意: 方法签名格式为 "methodName(Type1,Type2)"
    pub methods: HashMap<(String, String), MethodInfo>,
    /// (类名, 方法名) -> 方法签名列表 (用于查找重载)
    method_index: HashMap<(String, String), Vec<String>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 合并另一个 SymbolTable (用于 Rayon 并行 reduce)
    /// 
    /// v9.4: 支持并行构建符号表，避免串行合并瓶颈
    pub fn merge(&mut self, other: Self) {
        self.classes.extend(other.classes);
        self.fields.extend(other.fields);
        self.methods.extend(other.methods);
        // 合并方法索引
        for (key, sigs) in other.method_index {
            self.method_index.entry(key).or_default().extend(sigs);
        }
        // 合并 simple_name_index
        for (simple_name, fqns) in other.simple_name_index {
            let entry = self.simple_name_index.entry(simple_name).or_default();
            for fqn in fqns {
                if !entry.contains(&fqn) {
                    entry.push(fqn);
                }
            }
        }
    }

    /// Register class with FQN as primary key
    /// 
    /// This method properly handles classes with the same simple name but different packages
    /// by using the FQN as the primary key and maintaining a reverse lookup index.
    /// 
    /// # Arguments
    /// * `info` - TypeInfo with fqn field properly set
    pub fn register_class_fqn(&mut self, info: TypeInfo) {
        let fqn = info.fqn.clone();
        let simple_name = info.name.clone();
        
        // Insert into classes map with FQN as key
        self.classes.insert(fqn.clone(), info);
        
        // Update simple_name_index for reverse lookup
        let entry = self.simple_name_index.entry(simple_name).or_default();
        if !entry.contains(&fqn) {
            entry.push(fqn);
        }
    }

    /// 注册字段
    pub fn register_field(&mut self, class: &str, binding: VarBinding) {
        self.fields.insert((class.to_string(), binding.name.clone()), binding);
    }

    /// 查询变量的类型信息
    pub fn lookup_var_type(&self, class: &str, var_name: &str) -> Option<&TypeInfo> {
        // 先查字段
        if let Some(binding) = self.fields.get(&(class.to_string(), var_name.to_string())) {
            return self.classes.get(&binding.type_name);
        }
        None
    }
    
    /// 判断变量是否是 DAO 类型
    pub fn is_dao_var(&self, class: &str, var_name: &str) -> bool {
        if let Some(type_info) = self.lookup_var_type(class, var_name) {
            return type_info.is_dao();
        }
        // 退化到名称猜测
        var_name.ends_with("Repository") 
            || var_name.ends_with("Dao") 
            || var_name.ends_with("Mapper")
            || var_name.contains("repository")
            || var_name.contains("dao")
    }
    
    /// 判断方法调用是否是 DAO 操作
    pub fn is_dao_call(&self, class: &str, receiver: &str, method: &str) -> bool {
        // 1. 检查接收者类型
        if self.is_dao_var(class, receiver) {
            return true;
        }
        
        // 2. 检查方法名模式（DAO 常见方法）
        let dao_methods = [
            "find", "save", "delete", "update", "insert", "select",
            "getById", "findById", "findAll", "findOne",
            "saveAll", "deleteById", "deleteAll",
            "execute", "query", "count",
        ];
        
        for pattern in dao_methods {
            if method.starts_with(pattern) || method.contains(pattern) {
                return true;
            }
        }
        
        false
    }

    /// Lookup class by FQN (Fully Qualified Name)
    /// 
    /// # Arguments
    /// * `fqn` - The fully qualified class name (e.g., "com.example.service.UserService")
    /// 
    /// # Returns
    /// Reference to TypeInfo if found
    #[allow(dead_code)]
    pub fn lookup_by_fqn(&self, fqn: &str) -> Option<&TypeInfo> {
        self.classes.get(fqn)
    }

    /// Lookup classes by simple name
    /// 
    /// Returns all classes with the given simple name (may be in different packages)
    /// 
    /// # Arguments
    /// * `simple_name` - The simple class name (e.g., "UserService")
    /// 
    /// # Returns
    /// Vector of references to TypeInfo for all matching classes
    #[allow(dead_code)]
    pub fn lookup_by_simple_name(&self, simple_name: &str) -> Vec<&TypeInfo> {
        if let Some(fqns) = self.simple_name_index.get(simple_name) {
            fqns.iter()
                .filter_map(|fqn| self.classes.get(fqn))
                .collect()
        } else {
            // Fallback: check if simple_name is used as key directly (legacy support)
            self.classes.get(simple_name).into_iter().collect()
        }
    }

    /// Register a class (legacy method for backward compatibility)
    /// 
    /// Note: Prefer `register_class_fqn` for new code as it properly handles FQN
    #[allow(dead_code)]
    pub fn register_class(&mut self, info: TypeInfo) {
        let key = if info.fqn.is_empty() || info.fqn == info.name {
            info.name.clone()
        } else {
            info.fqn.clone()
        };
        let simple_name = info.name.clone();
        
        self.classes.insert(key.clone(), info);
        
        // Update simple_name_index
        let entry = self.simple_name_index.entry(simple_name).or_default();
        if !entry.contains(&key) {
            entry.push(key);
        }
    }

    /// Register a method
    /// 
    /// # Arguments
    /// * `class` - The class name (simple or FQN)
    /// * `method` - The method info to register
    #[allow(dead_code)]
    pub fn register_method(&mut self, class: &str, method: MethodInfo) {
        let sig = format!("{}({})", method.name, 
            method.params.iter().map(|p| p.type_name.as_str()).collect::<Vec<_>>().join(","));
        
        // Add to method_index for lookup by name
        self.method_index
            .entry((class.to_string(), method.name.clone()))
            .or_default()
            .push(sig.clone());
        
        // Add to methods map with signature as key
        self.methods.insert((class.to_string(), sig), method);
    }

    /// Lookup methods by name (returns all overloads)
    /// 
    /// # Arguments
    /// * `class` - The class name
    /// * `method_name` - The method name
    /// 
    /// # Returns
    /// Vector of references to MethodInfo for all overloads
    #[allow(dead_code)]
    pub fn lookup_methods(&self, class: &str, method_name: &str) -> Vec<&MethodInfo> {
        if let Some(sigs) = self.method_index.get(&(class.to_string(), method_name.to_string())) {
            sigs.iter()
                .filter_map(|sig| self.methods.get(&(class.to_string(), sig.clone())))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Lookup method by exact signature
    /// 
    /// # Arguments
    /// * `class` - The class name
    /// * `signature` - The method signature (e.g., "find(Long)")
    /// 
    /// # Returns
    /// Reference to MethodInfo if found
    #[allow(dead_code)]
    pub fn lookup_method_by_sig(&self, class: &str, signature: &str) -> Option<&MethodInfo> {
        self.methods.get(&(class.to_string(), signature.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // ImportIndex Unit Tests
    // ========================================================================

    #[test]
    fn test_import_index_from_imports_explicit() {
        let imports = vec![
            "com.example.repo.UserRepository".to_string(),
            "java.util.List".to_string(),
        ];
        let index = ImportIndex::from_imports(imports, Some("com.example.service".to_string()));

        assert_eq!(index.explicit.len(), 2);
        assert_eq!(
            index.explicit.get("UserRepository"),
            Some(&"com.example.repo.UserRepository".to_string())
        );
        assert_eq!(
            index.explicit.get("List"),
            Some(&"java.util.List".to_string())
        );
        assert!(index.wildcards.is_empty());
    }

    #[test]
    fn test_import_index_from_imports_wildcard() {
        let imports = vec![
            "com.example.repo.*".to_string(),
            "java.util.*".to_string(),
        ];
        let index = ImportIndex::from_imports(imports, None);

        assert!(index.explicit.is_empty());
        assert_eq!(index.wildcards.len(), 2);
        assert!(index.wildcards.contains(&"com.example.repo".to_string()));
        assert!(index.wildcards.contains(&"java.util".to_string()));
    }

    #[test]
    fn test_import_index_from_imports_mixed() {
        let imports = vec![
            "com.example.repo.UserRepository".to_string(),
            "java.util.*".to_string(),
            "com.example.service.OrderService".to_string(),
        ];
        let index = ImportIndex::from_imports(imports, Some("com.example.controller".to_string()));

        assert_eq!(index.explicit.len(), 2);
        assert_eq!(index.wildcards.len(), 1);
        assert_eq!(index.package, Some("com.example.controller".to_string()));
    }

    #[test]
    fn test_import_index_resolve_explicit() {
        let imports = vec!["com.example.repo.UserRepository".to_string()];
        let index = ImportIndex::from_imports(imports, None);
        let known_classes = HashMap::new();

        let resolved = index.resolve("UserRepository", &known_classes);
        assert_eq!(resolved, Some("com.example.repo.UserRepository".to_string()));
    }

    #[test]
    fn test_import_index_resolve_wildcard() {
        let imports = vec!["com.example.repo.*".to_string()];
        let index = ImportIndex::from_imports(imports, None);
        let mut known_classes = HashMap::new();
        known_classes.insert(
            "com.example.repo.UserRepository".to_string(),
            "UserRepository".to_string(),
        );

        let resolved = index.resolve("UserRepository", &known_classes);
        assert_eq!(resolved, Some("com.example.repo.UserRepository".to_string()));
    }

    #[test]
    fn test_import_index_resolve_same_package() {
        let index = ImportIndex::from_imports(vec![], Some("com.example.service".to_string()));
        let mut known_classes = HashMap::new();
        known_classes.insert(
            "com.example.service.UserService".to_string(),
            "UserService".to_string(),
        );

        let resolved = index.resolve("UserService", &known_classes);
        assert_eq!(resolved, Some("com.example.service.UserService".to_string()));
    }

    #[test]
    fn test_import_index_resolve_java_lang() {
        let index = ImportIndex::from_imports(vec![], None);
        let known_classes = HashMap::new();

        let resolved = index.resolve("String", &known_classes);
        assert_eq!(resolved, Some("java.lang.String".to_string()));

        let resolved = index.resolve("Integer", &known_classes);
        assert_eq!(resolved, Some("java.lang.Integer".to_string()));
    }

    #[test]
    fn test_import_index_resolve_priority() {
        // Explicit import should take priority over wildcard
        let imports = vec![
            "com.other.UserRepository".to_string(),  // explicit
            "com.example.repo.*".to_string(),        // wildcard
        ];
        let index = ImportIndex::from_imports(imports, None);
        let mut known_classes = HashMap::new();
        known_classes.insert(
            "com.example.repo.UserRepository".to_string(),
            "UserRepository".to_string(),
        );

        let resolved = index.resolve("UserRepository", &known_classes);
        // Should resolve to explicit import, not wildcard
        assert_eq!(resolved, Some("com.other.UserRepository".to_string()));
    }

    #[test]
    fn test_import_index_add_local_class() {
        let mut index = ImportIndex::from_imports(vec![], Some("com.example.service".to_string()));
        index.add_local_class("UserService");
        index.add_local_class("UserServiceImpl");
        index.add_local_class("UserService"); // duplicate

        assert_eq!(index.local_classes.len(), 2);
        assert!(index.local_classes.contains(&"UserService".to_string()));
        assert!(index.local_classes.contains(&"UserServiceImpl".to_string()));
    }

    #[test]
    fn test_import_index_resolve_local_class() {
        let mut index = ImportIndex::from_imports(vec![], Some("com.example.service".to_string()));
        index.add_local_class("UserService");
        let known_classes = HashMap::new();

        let resolved = index.resolve("UserService", &known_classes);
        assert_eq!(resolved, Some("com.example.service.UserService".to_string()));
    }

    // ========================================================================
    // ImportIndex Property Tests
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

    /// Strategy to generate explicit import statements
    fn explicit_import_strategy() -> impl Strategy<Value = String> {
        (java_package_strategy(), java_class_name_strategy())
            .prop_map(|(pkg, class)| format!("{}.{}", pkg, class))
    }

    /// Strategy to generate wildcard import statements
    fn wildcard_import_strategy() -> impl Strategy<Value = String> {
        java_package_strategy().prop_map(|pkg| format!("{}.*", pkg))
    }

    /// Strategy to generate a mix of explicit and wildcard imports
    fn mixed_imports_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                explicit_import_strategy(),
                wildcard_import_strategy(),
            ],
            0..=10
        )
    }

    proptest! {
        /// **Feature: java-perf-semantic-analysis, Property 1: Import Extraction Completeness**
        /// 
        /// *For any* valid Java file with N import statements, parsing SHALL extract 
        /// exactly N imports with correct classification (explicit vs wildcard).
        /// 
        /// **Validates: Requirements 1.1**
        #[test]
        fn prop_import_extraction_completeness(
            imports in mixed_imports_strategy(),
            package in prop::option::of(java_package_strategy())
        ) {
            let index = ImportIndex::from_imports(imports.clone(), package);

            // Count expected explicit and wildcard imports
            let expected_explicit: Vec<_> = imports.iter()
                .filter(|i| !i.ends_with(".*") && !i.is_empty())
                .collect();
            let expected_wildcards: Vec<_> = imports.iter()
                .filter(|i| i.ends_with(".*"))
                .collect();

            // Property: Total imports extracted equals input count
            // Note: explicit imports may have duplicates (same simple name), so we check >= 
            // because HashMap deduplicates by simple name
            prop_assert!(
                index.explicit.len() <= expected_explicit.len(),
                "Explicit imports should not exceed input count"
            );
            prop_assert_eq!(
                index.wildcards.len(),
                expected_wildcards.len(),
                "Wildcard count should match exactly"
            );

            // Property: Every explicit import maps simple name to full FQN
            for (simple_name, fqn) in &index.explicit {
                prop_assert!(
                    fqn.ends_with(simple_name),
                    "FQN '{}' should end with simple name '{}'",
                    fqn, simple_name
                );
                prop_assert!(
                    fqn.contains('.'),
                    "FQN '{}' should contain package separator",
                    fqn
                );
            }

            // Property: Every wildcard is a valid package (no .* suffix stored)
            for wildcard in &index.wildcards {
                prop_assert!(
                    !wildcard.ends_with(".*"),
                    "Wildcard '{}' should not contain .* suffix",
                    wildcard
                );
                prop_assert!(
                    !wildcard.is_empty(),
                    "Wildcard package should not be empty"
                );
            }
        }

        /// **Feature: java-perf-semantic-analysis, Property 2: FQN Resolution Priority**
        /// 
        /// *For any* simple class name and import context, resolution SHALL follow the 
        /// priority order: explicit imports → wildcard imports → same-package → java.lang,
        /// and return the first match.
        /// 
        /// **Validates: Requirements 1.2, 1.5**
        #[test]
        fn prop_fqn_resolution_priority(
            class_name in java_class_name_strategy(),
            explicit_pkg in java_package_strategy(),
            wildcard_pkg in java_package_strategy(),
            same_pkg in java_package_strategy(),
        ) {
            // Setup: Create an ImportIndex with all resolution paths available
            let explicit_fqn = format!("{}.{}", explicit_pkg, class_name);
            let wildcard_fqn = format!("{}.{}", wildcard_pkg, class_name);
            let same_pkg_fqn = format!("{}.{}", same_pkg, class_name);

            // Create imports with explicit import for the class
            let imports = vec![
                explicit_fqn.clone(),           // explicit import
                format!("{}.*", wildcard_pkg),  // wildcard import
            ];
            let index = ImportIndex::from_imports(imports, Some(same_pkg.clone()));

            // Create known_classes with entries for wildcard and same-package resolution
            let mut known_classes = HashMap::new();
            known_classes.insert(wildcard_fqn.clone(), class_name.clone());
            known_classes.insert(same_pkg_fqn.clone(), class_name.clone());

            // Property 1: Explicit import takes priority over everything
            let resolved = index.resolve(&class_name, &known_classes);
            prop_assert_eq!(
                resolved.as_ref(),
                Some(&explicit_fqn),
                "Explicit import should take priority. Got {:?}, expected {:?}",
                resolved, explicit_fqn
            );

            // Property 2: Without explicit import, wildcard takes priority over same-package
            let imports_no_explicit = vec![format!("{}.*", wildcard_pkg)];
            let index_no_explicit = ImportIndex::from_imports(imports_no_explicit, Some(same_pkg.clone()));
            let resolved_no_explicit = index_no_explicit.resolve(&class_name, &known_classes);
            prop_assert_eq!(
                resolved_no_explicit.as_ref(),
                Some(&wildcard_fqn),
                "Wildcard import should take priority over same-package. Got {:?}, expected {:?}",
                resolved_no_explicit, wildcard_fqn
            );

            // Property 3: Without explicit or wildcard, same-package is used
            let index_same_pkg_only = ImportIndex::from_imports(vec![], Some(same_pkg.clone()));
            let resolved_same_pkg = index_same_pkg_only.resolve(&class_name, &known_classes);
            prop_assert_eq!(
                resolved_same_pkg.as_ref(),
                Some(&same_pkg_fqn),
                "Same-package should be used when no imports match. Got {:?}, expected {:?}",
                resolved_same_pkg, same_pkg_fqn
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 2 (continued): java.lang fallback**
        /// 
        /// *For any* java.lang class name, resolution SHALL return java.lang.{ClassName}
        /// when no other imports match.
        /// 
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_java_lang_fallback(
            other_pkg in java_package_strategy(),
        ) {
            // Test with common java.lang classes
            let java_lang_classes = vec!["String", "Integer", "Long", "Object", "Exception"];
            
            for class_name in java_lang_classes {
                // Create an ImportIndex with no matching imports
                let index = ImportIndex::from_imports(vec![], Some(other_pkg.clone()));
                let known_classes = HashMap::new();

                let resolved = index.resolve(class_name, &known_classes);
                let expected = format!("java.lang.{}", class_name);
                
                prop_assert_eq!(
                    resolved.as_ref(),
                    Some(&expected),
                    "java.lang.{} should be resolved as fallback. Got {:?}",
                    class_name, resolved
                );
            }
        }

        /// **Feature: java-perf-semantic-analysis, Property 3: FQN Uniqueness in SymbolTable**
        /// 
        /// *For any* two classes with the same simple name but different packages, they SHALL 
        /// be stored as separate entries in SymbolTable keyed by their distinct FQNs.
        /// 
        /// **Validates: Requirements 1.3**
        #[test]
        fn prop_fqn_uniqueness_in_symbol_table(
            class_name in java_class_name_strategy(),
            pkg1 in java_package_strategy(),
            pkg2 in java_package_strategy(),
        ) {
            // Ensure packages are different
            prop_assume!(pkg1 != pkg2);

            let mut table = SymbolTable::new();

            // Create two classes with the same simple name but different packages
            let type1 = TypeInfo::new_with_package(
                &class_name,
                Some(&pkg1),
                PathBuf::from("File1.java"),
                1,
            );
            let type2 = TypeInfo::new_with_package(
                &class_name,
                Some(&pkg2),
                PathBuf::from("File2.java"),
                1,
            );

            let fqn1 = type1.fqn.clone();
            let fqn2 = type2.fqn.clone();

            // Register both classes using FQN-based registration
            table.register_class_fqn(type1);
            table.register_class_fqn(type2);

            // Property 1: Both classes should be stored (2 entries in classes map)
            prop_assert_eq!(
                table.classes.len(),
                2,
                "SymbolTable should contain 2 classes with same simple name but different FQNs"
            );

            // Property 2: Each class should be retrievable by its FQN
            prop_assert!(
                table.lookup_by_fqn(&fqn1).is_some(),
                "Class with FQN '{}' should be retrievable",
                fqn1
            );
            prop_assert!(
                table.lookup_by_fqn(&fqn2).is_some(),
                "Class with FQN '{}' should be retrievable",
                fqn2
            );

            // Property 3: FQNs should be distinct
            prop_assert_ne!(
                &fqn1, &fqn2,
                "FQNs should be different for classes in different packages"
            );

            // Property 4: simple_name_index should map simple name to both FQNs
            let fqns_for_simple_name = table.simple_name_index.get(&class_name);
            prop_assert!(
                fqns_for_simple_name.is_some(),
                "simple_name_index should contain entry for '{}'",
                class_name
            );
            let fqns = fqns_for_simple_name.unwrap();
            prop_assert_eq!(
                fqns.len(),
                2,
                "simple_name_index should map '{}' to 2 FQNs",
                class_name
            );
            prop_assert!(
                fqns.contains(&fqn1),
                "simple_name_index should contain FQN '{}'",
                fqn1
            );
            prop_assert!(
                fqns.contains(&fqn2),
                "simple_name_index should contain FQN '{}'",
                fqn2
            );

            // Property 5: lookup_by_simple_name should return both classes
            let classes = table.lookup_by_simple_name(&class_name);
            prop_assert_eq!(
                classes.len(),
                2,
                "lookup_by_simple_name('{}') should return 2 classes",
                class_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 13: ImportIndex Merge Isolation**
        /// 
        /// *For any* two ImportIndex instances from different files, merging them into a global 
        /// context SHALL NOT cause one file's imports to affect another file's resolution.
        /// 
        /// **Validates: Requirements 5.3**
        #[test]
        fn prop_import_index_merge_isolation(
            // File 1 setup
            class1 in java_class_name_strategy(),
            pkg1 in java_package_strategy(),
            import1_class in java_class_name_strategy(),
            import1_pkg in java_package_strategy(),
            // File 2 setup
            class2 in java_class_name_strategy(),
            pkg2 in java_package_strategy(),
            import2_class in java_class_name_strategy(),
            import2_pkg in java_package_strategy(),
        ) {
            // Ensure packages are different to avoid collisions
            prop_assume!(pkg1 != pkg2);
            prop_assume!(import1_pkg != import2_pkg);
            prop_assume!(class1 != class2);
            prop_assume!(import1_class != import2_class);
            // Ensure local classes don't conflict with explicit imports
            // (explicit imports take priority, which would make the test assertions invalid)
            prop_assume!(class1 != import1_class);
            prop_assume!(class2 != import2_class);

            // Create ImportIndex for File 1
            let mut index1 = ImportIndex::from_imports(
                vec![format!("{}.{}", import1_pkg, import1_class)],
                Some(pkg1.clone()),
            );
            index1.add_local_class(&class1);

            // Create ImportIndex for File 2
            let mut index2 = ImportIndex::from_imports(
                vec![format!("{}.{}", import2_pkg, import2_class)],
                Some(pkg2.clone()),
            );
            index2.add_local_class(&class2);

            // Build global PackageClassIndex from both ImportIndex instances
            let mut import_indices = HashMap::new();
            import_indices.insert("File1.java".to_string(), index1.clone());
            import_indices.insert("File2.java".to_string(), index2.clone());
            
            let package_class_index = PackageClassIndex::from_import_indices(&import_indices);
            let known_classes = package_class_index.to_known_classes();

            // Property 1: File 1's explicit import should NOT be visible in File 2's resolution
            // File 2 should NOT be able to resolve import1_class via its own ImportIndex
            let resolved_in_file2 = index2.resolve(&import1_class, &known_classes);
            // If import1_class is resolved, it should NOT be from import1_pkg (File 1's import)
            // unless it happens to be in File 2's package or wildcards
            if let Some(ref resolved) = resolved_in_file2 {
                // The resolved FQN should NOT be the explicit import from File 1
                let file1_explicit_fqn = format!("{}.{}", import1_pkg, import1_class);
                prop_assert_ne!(
                    resolved, &file1_explicit_fqn,
                    "File 2 should NOT resolve '{}' to File 1's explicit import '{}'",
                    import1_class, file1_explicit_fqn
                );
            }

            // Property 2: File 2's explicit import should NOT be visible in File 1's resolution
            let resolved_in_file1 = index1.resolve(&import2_class, &known_classes);
            if let Some(ref resolved) = resolved_in_file1 {
                let file2_explicit_fqn = format!("{}.{}", import2_pkg, import2_class);
                prop_assert_ne!(
                    resolved, &file2_explicit_fqn,
                    "File 1 should NOT resolve '{}' to File 2's explicit import '{}'",
                    import2_class, file2_explicit_fqn
                );
            }

            // Property 3: Each file's local class should be resolvable only within its own scope
            // File 1 should resolve class1 to pkg1.class1
            let resolved_local1 = index1.resolve(&class1, &known_classes);
            let expected_fqn1 = format!("{}.{}", pkg1, class1);
            prop_assert_eq!(
                resolved_local1.as_ref(),
                Some(&expected_fqn1),
                "File 1 should resolve local class '{}' to '{}'",
                class1, expected_fqn1
            );

            // File 2 should resolve class2 to pkg2.class2
            let resolved_local2 = index2.resolve(&class2, &known_classes);
            let expected_fqn2 = format!("{}.{}", pkg2, class2);
            prop_assert_eq!(
                resolved_local2.as_ref(),
                Some(&expected_fqn2),
                "File 2 should resolve local class '{}' to '{}'",
                class2, expected_fqn2
            );

            // Property 4: File 1 should NOT resolve File 2's local class via same-package rules
            // (unless they happen to be in the same package, which we excluded)
            let resolved_class2_in_file1 = index1.resolve(&class2, &known_classes);
            // class2 should only be resolvable if it's in known_classes (from PackageClassIndex)
            // but NOT via File 1's same-package resolution
            if let Some(ref resolved) = resolved_class2_in_file1 {
                // If resolved, it should be from the global index, not from pkg1
                let wrong_fqn = format!("{}.{}", pkg1, class2);
                prop_assert_ne!(
                    resolved, &wrong_fqn,
                    "File 1 should NOT resolve '{}' to its own package '{}'",
                    class2, wrong_fqn
                );
            }

            // Property 5: PackageClassIndex should contain both local classes
            prop_assert!(
                package_class_index.fqn_to_simple.contains_key(&expected_fqn1),
                "PackageClassIndex should contain FQN '{}'",
                expected_fqn1
            );
            prop_assert!(
                package_class_index.fqn_to_simple.contains_key(&expected_fqn2),
                "PackageClassIndex should contain FQN '{}'",
                expected_fqn2
            );

            // Property 6: Original ImportIndex instances should remain unchanged
            // (they should still have their original explicit imports)
            prop_assert_eq!(
                index1.explicit.len(), 1,
                "File 1's ImportIndex should still have exactly 1 explicit import"
            );
            prop_assert_eq!(
                index2.explicit.len(), 1,
                "File 2's ImportIndex should still have exactly 1 explicit import"
            );
            prop_assert_eq!(
                index1.package.as_ref(), Some(&pkg1),
                "File 1's package should remain unchanged"
            );
            prop_assert_eq!(
                index2.package.as_ref(), Some(&pkg2),
                "File 2's package should remain unchanged"
            );
        }
    }
    
    #[test]
    fn test_layer_from_annotation() {
        assert_eq!(LayerType::from_annotation("Repository"), LayerType::Repository);
        assert_eq!(LayerType::from_annotation("RestController"), LayerType::Controller);
        assert_eq!(LayerType::from_annotation("Service"), LayerType::Service);
    }
    
    #[test]
    fn test_is_dao_type() {
        let mut type_info = TypeInfo::new("UserRepository", PathBuf::from("test.java"), 1);
        assert!(type_info.is_dao()); // 基于名称
        
        type_info.add_annotation("Repository");
        assert!(type_info.is_dao()); // 基于注解
        assert_eq!(type_info.layer, LayerType::Repository);
    }
    
    #[test]
    fn test_symbol_table_lookup() {
        let mut table = SymbolTable::new();
        
        // 注册 Repository 类
        let mut repo_type = TypeInfo::new("UserRepository", PathBuf::from("UserRepository.java"), 1);
        repo_type.add_annotation("Repository");
        table.register_class(repo_type);
        
        // 注册字段
        let binding = VarBinding::new("userRepo", "UserRepository", true);
        table.register_field("UserService", binding);
        
        // 测试查询
        assert!(table.is_dao_var("UserService", "userRepo"));
        assert!(table.is_dao_call("UserService", "userRepo", "findById"));
    }

    #[test]
    fn test_method_overload() {
        let mut table = SymbolTable::new();

        // 注册两个重载方法
        let mut find_by_id = MethodInfo::new("find", "UserRepository", 10);
        find_by_id.add_param("id", "Long");
        find_by_id.return_type = Some("User".to_string());

        let mut find_by_name = MethodInfo::new("find", "UserRepository", 15);
        find_by_name.add_param("name", "String");
        find_by_name.return_type = Some("User".to_string());

        table.register_method("UserRepository", find_by_id);
        table.register_method("UserRepository", find_by_name);

        // 按名称查找应返回两个方法
        let methods = table.lookup_methods("UserRepository", "find");
        assert_eq!(methods.len(), 2);

        // 按签名查找应返回精确匹配
        let method = table.lookup_method_by_sig("UserRepository", "find(Long)");
        assert!(method.is_some());
        assert_eq!(method.unwrap().params[0].type_name, "Long");

        let method2 = table.lookup_method_by_sig("UserRepository", "find(String)");
        assert!(method2.is_some());
        assert_eq!(method2.unwrap().params[0].type_name, "String");
    }

    #[test]
    fn test_method_signature() {
        let mut method = MethodInfo::new("save", "UserRepository", 20);
        method.add_param("user", "User");
        method.add_param("flush", "boolean");

        assert_eq!(method.signature(), "save(User,boolean)");
    }

    #[test]
    fn test_symbol_table_merge() {
        // 创建第一个表
        let mut table1 = SymbolTable::new();
        let mut repo_type = TypeInfo::new("UserRepository", PathBuf::from("UserRepository.java"), 1);
        repo_type.add_annotation("Repository");
        table1.register_class(repo_type);
        table1.register_field("UserService", VarBinding::new("userRepo", "UserRepository", true));

        // 创建第二个表
        let mut table2 = SymbolTable::new();
        let service_type = TypeInfo::new("OrderService", PathBuf::from("OrderService.java"), 1);
        table2.register_class(service_type);
        table2.register_field("OrderController", VarBinding::new("orderService", "OrderService", true));

        // 合并
        table1.merge(table2);

        // 验证合并结果
        assert_eq!(table1.classes.len(), 2);
        assert!(table1.classes.contains_key("UserRepository"));
        assert!(table1.classes.contains_key("OrderService"));
        assert_eq!(table1.fields.len(), 2);
        assert!(table1.is_dao_var("UserService", "userRepo"));
    }
}
