# Design Document: Java-Perf Semantic Analysis Enhancement

## Overview

This design document describes the architectural changes required to enhance the `java-perf` static analysis tool with proper Fully Qualified Name (FQN) resolution, improved ThreadLocal leak detection, and structured project detection. The core goal is to transform the analyzer from a heuristic-based tool into a semantically-aware static analysis engine.

## Architecture

The enhancement follows a layered architecture that builds upon the existing two-phase scanning model:

```
┌─────────────────────────────────────────────────────────────┐
│                      Phase 1: Indexing                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ ImportIndex │  │ SymbolTable │  │     CallGraph       │  │
│  │  (per-file) │  │  (global)   │  │     (global)        │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                    FQN Resolution                            │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Phase 2: Analysis                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │  Rule Handlers  │  │ Project Detector│                   │
│  │ (with FQN ctx)  │  │ (structured XML)│                   │
│  └─────────────────┘  └─────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### 1. ImportIndex (New Component)

A per-file data structure for efficient import resolution.

```rust
/// Import resolution index for a single Java file
#[derive(Debug, Clone, Default)]
pub struct ImportIndex {
    /// Explicit imports: "UserRepository" -> "com.example.repo.UserRepository"
    pub explicit: HashMap<String, String>,
    /// Wildcard import packages: ["com.example.repo", "java.util"]
    pub wildcards: Vec<String>,
    /// Current file's package: "com.example.service"
    pub package: Option<String>,
    /// Classes defined in this file (auto-imported)
    pub local_classes: Vec<String>,
}

impl ImportIndex {
    /// Resolve a simple class name to FQN
    pub fn resolve(&self, simple_name: &str, known_classes: &HashMap<String, String>) -> Option<String>;
    
    /// Build from parsed import statements
    pub fn from_imports(imports: Vec<String>, package: Option<String>) -> Self;
}
```

### 2. Enhanced SymbolTable

Extend the existing SymbolTable to use FQNs as primary keys.

```rust
/// Enhanced SymbolTable with FQN support
pub struct SymbolTable {
    /// FQN -> TypeInfo (e.g., "com.example.repo.UserRepository" -> TypeInfo)
    pub classes: HashMap<String, TypeInfo>,
    /// (FQN, field_name) -> VarBinding
    pub fields: HashMap<(String, String), VarBinding>,
    /// Simple name -> Vec<FQN> (for ambiguity detection)
    pub simple_name_index: HashMap<String, Vec<String>>,
    // ... existing fields
}

impl SymbolTable {
    /// Register class with FQN
    pub fn register_class_fqn(&mut self, fqn: &str, info: TypeInfo);
    
    /// Lookup by FQN (primary)
    pub fn lookup_by_fqn(&self, fqn: &str) -> Option<&TypeInfo>;
    
    /// Lookup by simple name (may return multiple)
    pub fn lookup_by_simple_name(&self, name: &str) -> Vec<&TypeInfo>;
    
    /// Resolve field type to FQN
    pub fn resolve_field_type_fqn(&self, class_fqn: &str, field_name: &str, import_index: &ImportIndex) -> Option<String>;
}
```

### 3. Enhanced CallGraph

Update CallGraph to use FQN-based method signatures.

```rust
/// Method signature with FQN
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MethodSig {
    /// FQN of the class: "com.example.service.UserService"
    pub class_fqn: String,
    /// Method name: "findById"
    pub name: String,
}

impl MethodSig {
    pub fn new_fqn(class_fqn: &str, name: &str) -> Self;
    
    /// Create from simple name with import resolution
    pub fn resolve(simple_class: &str, name: &str, import_index: &ImportIndex, symbol_table: &SymbolTable) -> Self;
}
```

### 4. ThreadLocalLeakHandler Enhancement

Improve detection to verify finally block placement.

```rust
impl ThreadLocalLeakHandler {
    /// Check if remove() is in a finally block
    fn has_remove_in_finally(&self, method_node: Node, var_name: &str, code: &str) -> bool {
        // 1. Find all try_statement nodes in method
        // 2. For each try_statement, check finally_clause
        // 3. In finally_clause, look for method_invocation matching var_name.remove()
    }
    
    /// Determine severity based on remove() placement
    fn determine_severity(&self, has_finally_remove: bool, has_any_remove: bool) -> Severity {
        match (has_finally_remove, has_any_remove) {
            (true, _) => return None, // Safe, no issue
            (false, true) => Severity::P1, // Remove exists but not in finally
            (false, false) => Severity::P0, // No remove at all
        }
    }
}
```

### 5. Structured Project Detector

Replace string matching with proper XML/Gradle parsing.

```rust
/// Parsed Maven dependency
#[derive(Debug)]
pub struct MavenDependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: DependencyScope,
}

#[derive(Debug, PartialEq)]
pub enum DependencyScope {
    Compile,
    Test,
    Runtime,
    Provided,
}

impl ProjectDetector {
    /// Parse pom.xml using quick-xml
    pub fn parse_maven_pom(content: &str) -> Result<Vec<MavenDependency>>;
    
    /// Parse build.gradle
    pub fn parse_gradle_build(content: &str) -> Result<Vec<GradleDependency>>;
    
    /// Detect stack from parsed dependencies (excluding test scope)
    pub fn detect_stack_from_deps(deps: &[MavenDependency]) -> DetectedStack;
}
```

## Data Models

### ImportIndex

```rust
pub struct ImportIndex {
    pub explicit: HashMap<String, String>,  // O(1) lookup
    pub wildcards: Vec<String>,             // Linear scan for wildcards
    pub package: Option<String>,
    pub local_classes: Vec<String>,
}
```

### Enhanced TypeInfo

```rust
pub struct TypeInfo {
    pub name: String,           // Simple name: "UserRepository"
    pub fqn: String,            // Full name: "com.example.repo.UserRepository"
    pub package: Option<String>,
    pub annotations: Vec<String>,
    pub layer: LayerType,
    pub file: PathBuf,
    pub line: usize,
}
```

### MavenDependency

```rust
pub struct MavenDependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: DependencyScope,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Import Extraction Completeness
*For any* valid Java file with N import statements, parsing SHALL extract exactly N imports with correct classification (explicit vs wildcard).
**Validates: Requirements 1.1**

### Property 2: FQN Resolution Priority
*For any* simple class name and import context, resolution SHALL follow the priority order: explicit imports → wildcard imports → same-package → java.lang, and return the first match.
**Validates: Requirements 1.2, 1.5**

### Property 3: FQN Uniqueness in SymbolTable
*For any* two classes with the same simple name but different packages, they SHALL be stored as separate entries in SymbolTable keyed by their distinct FQNs.
**Validates: Requirements 1.3**

### Property 4: CallGraph FQN Format
*For any* MethodSig in the CallGraph, the class field SHALL contain a valid FQN (containing at least one dot separator) or be marked as unresolved.
**Validates: Requirements 1.4**

### Property 5: ThreadLocal Safe Detection
*For any* method containing ThreadLocal.set() followed by remove() in a finally block for the same variable, the system SHALL NOT report a leak issue.
**Validates: Requirements 2.1, 2.3, 2.4**

### Property 6: ThreadLocal Severity Gradation
*For any* ThreadLocal.set() without remove() in finally, the severity SHALL be P0 if no remove() exists anywhere, or P1 if remove() exists outside finally.
**Validates: Requirements 2.2**

### Property 7: Maven Scope Filtering
*For any* pom.xml with test-scoped dependencies, those dependencies SHALL NOT affect the detected project stack (is_spring_boot, is_reactive, etc.).
**Validates: Requirements 3.1, 3.2, 3.5**

### Property 8: XML Comment Handling
*For any* dependency element inside an XML comment in pom.xml, it SHALL NOT be included in the parsed dependency list.
**Validates: Requirements 3.3**

### Property 9: Gradle Configuration Distinction
*For any* build.gradle with testImplementation dependencies, those SHALL be classified with Test scope and not affect main stack detection.
**Validates: Requirements 3.4**

### Property 10: Field Type Resolution to CallGraph
*For any* method call on a field where the field's type is registered in SymbolTable, the resulting CallGraph edge SHALL use the resolved FQN.
**Validates: Requirements 4.1, 4.2**

### Property 11: Cross-Package Call Chain Tracing
*For any* call chain from a Controller class to a Repository class through Service classes in different packages, trace_to_layer() SHALL find the complete path.
**Validates: Requirements 4.3**

### Property 12: Heuristic Fallback Marking
*For any* field type that cannot be resolved via SymbolTable, the system SHALL use heuristic detection AND mark the result with reduced confidence.
**Validates: Requirements 4.4**

### Property 13: ImportIndex Merge Isolation
*For any* two ImportIndex instances from different files, merging them into a global context SHALL NOT cause one file's imports to affect another file's resolution.
**Validates: Requirements 5.3**

### Property 14: Local Class Auto-Registration
*For any* class defined in a Java file, it SHALL be automatically added to that file's ImportIndex with its package-qualified FQN.
**Validates: Requirements 5.4**

## Error Handling

1. **XML Parse Errors**: If pom.xml is malformed, fall back to string matching with a warning
2. **Unresolvable Types**: Mark as "unresolved" and use heuristic detection
3. **Circular Imports**: Detect and break cycles during resolution
4. **Missing Package Declaration**: Assume default package (empty string)

## Testing Strategy

### Dual Testing Approach

This implementation requires both unit tests and property-based tests:

- **Unit tests**: Verify specific examples, edge cases, and integration points
- **Property-based tests**: Verify universal properties across all valid inputs

### Property-Based Testing Framework

Use `proptest` crate for Rust property-based testing.

```toml
[dev-dependencies]
proptest = "1.4"
```

### Test Categories

1. **ImportIndex Tests**
   - Property tests for resolution priority
   - Unit tests for edge cases (empty imports, duplicate names)

2. **SymbolTable FQN Tests**
   - Property tests for uniqueness
   - Unit tests for lookup operations

3. **ThreadLocal Detection Tests**
   - Property tests for finally block detection
   - Unit tests for various code patterns

4. **Project Detector Tests**
   - Property tests for scope filtering
   - Unit tests for XML parsing edge cases

5. **Integration Tests**
   - End-to-end N+1 detection with cross-package classes
   - Full project scan with FQN resolution
