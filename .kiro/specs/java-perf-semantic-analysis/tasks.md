# Implementation Plan

## Phase 1: Core Data Structures

- [x] 1. Implement ImportIndex data structure
  - [x] 1.1 Create ImportIndex struct in symbol_table.rs
    - Add `explicit: HashMap<String, String>` for explicit imports
    - Add `wildcards: Vec<String>` for wildcard imports
    - Add `package: Option<String>` for current package
    - Add `local_classes: Vec<String>` for classes defined in file
    - _Requirements: 5.1_
  - [x] 1.2 Write property test for ImportIndex construction
    - **Property 1: Import Extraction Completeness**
    - **Validates: Requirements 1.1**
  - [x] 1.3 Implement ImportIndex::resolve() method
    - Priority: explicit → wildcards → same-package → java.lang
    - Return Option<String> for resolved FQN
    - _Requirements: 1.2, 1.5_
  - [x] 1.4 Write property test for FQN resolution priority
    - **Property 2: FQN Resolution Priority**
    - **Validates: Requirements 1.2, 1.5**
  - [x] 1.5 Implement ImportIndex::from_imports() builder
    - Parse import strings into explicit/wildcard categories
    - _Requirements: 1.1_

- [x] 2. Enhance SymbolTable with FQN support
  - [x] 2.1 Add fqn field to TypeInfo struct
    - Modify TypeInfo to include `fqn: String`
    - Update TypeInfo::new() to accept package parameter
    - _Requirements: 1.3_
  - [x] 2.2 Add simple_name_index to SymbolTable
    - Add `simple_name_index: HashMap<String, Vec<String>>` for reverse lookup
    - _Requirements: 1.3_
  - [x] 2.3 Implement register_class_fqn() method
    - Register class with FQN as primary key
    - Update simple_name_index for reverse lookup
    - _Requirements: 1.3_
  - [x] 2.4 Write property test for FQN uniqueness
    - **Property 3: FQN Uniqueness in SymbolTable**
    - **Validates: Requirements 1.3**
  - [x] 2.5 Implement resolve_field_type_fqn() method
    - Use ImportIndex to resolve field type to FQN
    - _Requirements: 4.1_

- [x] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Phase 2: Parser Integration

- [x] 4. Enhance JavaTreeSitterAnalyzer for package extraction
  - [x] 4.1 Add package declaration query
    - Create Tree-sitter query for `package_declaration`
    - Extract package name during Phase 1 parsing
    - _Requirements: 1.1_
  - [x] 4.2 Update extract_symbols() to return ImportIndex
    - Modify return type to include ImportIndex
    - Build ImportIndex from parsed imports and package
    - _Requirements: 1.1, 5.1_
  - [x] 4.3 Update extract_symbols_from_tree() to build FQN
    - Combine package + class name to create FQN
    - Set TypeInfo.fqn field
    - _Requirements: 1.3_
  - [x] 4.4 Write property test for local class auto-registration
    - **Property 14: Local Class Auto-Registration**
    - **Validates: Requirements 5.4**

- [x] 5. Update ast_engine.rs Phase 1 integration
  - [x] 5.1 Modify radar_scan() to collect ImportIndex per file
    - Store ImportIndex alongside SymbolTable during Phase 1
    - _Requirements: 5.1_
  - [x] 5.2 Implement ImportIndex merge strategy
    - Create global package→classes index from all ImportIndex
    - Maintain per-file import scopes
    - _Requirements: 5.3_
  - [x] 5.3 Write property test for ImportIndex merge isolation
    - **Property 13: ImportIndex Merge Isolation**
    - **Validates: Requirements 5.3**

- [x] 6. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Phase 3: CallGraph Enhancement

- [x] 7. Update CallGraph to use FQN
  - [x] 7.1 Modify MethodSig to use class_fqn
    - Rename `class` field to `class_fqn`
    - Update MethodSig::new() to MethodSig::new_fqn()
    - _Requirements: 1.4_
  - [x] 7.2 Implement MethodSig::resolve() with ImportIndex
    - Resolve simple class name to FQN using ImportIndex
    - Fall back to simple name if unresolvable (marked)
    - _Requirements: 1.4, 4.4_
  - [x] 7.3 Write property test for CallGraph FQN format
    - **Property 4: CallGraph FQN Format**
    - **Validates: Requirements 1.4**
  - [x] 7.4 Update extract_call_sites() to use FQN resolution
    - Pass ImportIndex to call site extraction
    - Resolve receiver types to FQN
    - _Requirements: 4.1, 4.2_
  - [x] 7.5 Write property test for field type resolution
    - **Property 10: Field Type Resolution to CallGraph**
    - **Validates: Requirements 4.1, 4.2**

- [x] 8. Update trace_to_layer() for cross-package tracing
  - [x] 8.1 Modify class_layers to use FQN keys
    - Update HashMap<String, LayerType> to use FQN
    - _Requirements: 4.3_
  - [x] 8.2 Update register_class() to use FQN
    - Accept FQN parameter instead of simple name
    - _Requirements: 4.3_
  - [x] 8.3 Write property test for cross-package call chain
    - **Property 11: Cross-Package Call Chain Tracing**
    - **Validates: Requirements 4.3**

- [x] 9. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Phase 4: ThreadLocal Detection Enhancement

- [x] 10. Enhance ThreadLocalLeakHandler
  - [x] 10.1 Implement has_remove_in_finally() method
    - Traverse AST to find try_statement nodes
    - Check finally_clause for matching remove() call
    - _Requirements: 2.1_
  - [x] 10.2 Implement determine_severity() method
    - Return None if remove in finally (safe)
    - Return P1 if remove outside finally
    - Return P0 if no remove at all
    - _Requirements: 2.2_
  - [x] 10.3 Update handle() to use new detection logic
    - Replace string contains check with AST traversal
    - Apply severity gradation
    - _Requirements: 2.1, 2.2, 2.3_
  - [x] 10.4 Write property test for ThreadLocal safe detection
    - **Property 5: ThreadLocal Safe Detection**
    - **Validates: Requirements 2.1, 2.3, 2.4**
  - [x] 10.5 Write property test for severity gradation
    - **Property 6: ThreadLocal Severity Gradation**
    - **Validates: Requirements 2.2**

- [x] 11. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Phase 5: Structured Project Detection

- [x] 12. Implement Maven XML parsing
  - [x] 12.1 Add quick-xml dependency to Cargo.toml
    - Add `quick-xml = "0.31"` to dependencies
    - _Requirements: 3.1_
  - [x] 12.2 Create MavenDependency and DependencyScope types
    - Define structs in project_detector.rs
    - _Requirements: 3.1_
  - [x] 12.3 Implement parse_maven_pom() function
    - Parse XML structure using quick-xml
    - Extract groupId, artifactId, version, scope
    - Handle XML comments (skip commented elements)
    - _Requirements: 3.1, 3.3_
  - [x] 12.4 Write property test for Maven scope filtering
    - **Property 7: Maven Scope Filtering**
    - **Validates: Requirements 3.1, 3.2, 3.5**
  - [x] 12.5 Write property test for XML comment handling
    - **Property 8: XML Comment Handling**
    - **Validates: Requirements 3.3**

- [x] 13. Implement Gradle parsing
  - [x] 13.1 Create GradleDependency type
    - Define struct with configuration field
    - _Requirements: 3.4_
  - [x] 13.2 Implement parse_gradle_build() function
    - Parse implementation, testImplementation, etc.
    - Use regex or simple parsing for Gradle DSL
    - _Requirements: 3.4_
  - [x] 13.3 Write property test for Gradle configuration distinction
    - **Property 9: Gradle Configuration Distinction**
    - **Validates: Requirements 3.4**

- [x] 14. Update detect_stack() to use parsed dependencies
  - [x] 14.1 Modify analyze_maven() to use parse_maven_pom()
    - Replace string contains with structured parsing
    - Filter out test-scoped dependencies
    - _Requirements: 3.1, 3.2_
  - [x] 14.2 Modify analyze_gradle() to use parse_gradle_build()
    - Replace string contains with structured parsing
    - Filter out testImplementation dependencies
    - _Requirements: 3.4_
  - [x] 14.3 Implement detect_stack_from_deps() function
    - Determine stack from filtered dependency list
    - _Requirements: 3.2, 3.5_

- [x] 15. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Phase 6: Heuristic Fallback and Integration

- [x] 16. Implement heuristic fallback with confidence marking
  - [x] 16.1 Add confidence field to Issue struct
    - Add `confidence: Option<Confidence>` enum (High, Medium, Low)
    - _Requirements: 4.4_
  - [x] 16.2 Update NPlusOneHandler to mark confidence
    - High confidence when FQN resolved
    - Low confidence when using heuristic fallback
    - _Requirements: 4.4_
  - [x] 16.3 Write property test for heuristic fallback marking
    - **Property 12: Heuristic Fallback Marking**
    - **Validates: Requirements 4.4**

- [x] 17. Integration testing
  - [x] 17.1 Write integration test for cross-package N+1 detection
    - Create test fixture with Controller→Service→Repository in different packages
    - Verify N+1 detection works across packages
    - _Requirements: 4.3_
  - [x] 17.2 Write integration test for full project scan
    - Use spring-boot-sample fixture
    - Verify FQN resolution and CallGraph accuracy
    - _Requirements: 1.2, 1.4, 4.2_

- [x] 18. Final Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
