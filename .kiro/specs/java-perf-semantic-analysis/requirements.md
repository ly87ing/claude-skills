# Requirements Document

## Introduction

This document specifies the requirements for enhancing the `java-perf` static analysis tool's semantic analysis capabilities. The current implementation relies on simple class names for symbol tracking, which leads to inaccurate cross-file analysis (especially N+1 detection). This enhancement will implement proper Fully Qualified Name (FQN) resolution, improve ThreadLocal leak detection accuracy, and enhance project detection reliability.

## Glossary

- **FQN (Fully Qualified Name)**: The complete class name including package path (e.g., `com.example.repository.UserRepository`)
- **Simple Class Name**: The class name without package path (e.g., `UserRepository`)
- **Import Resolution**: The process of mapping simple class names to their FQNs using import statements
- **SymbolTable**: A data structure that tracks type information and variable bindings across files
- **CallGraph**: A data structure representing method call relationships across the codebase
- **LayerType**: Classification of classes into architectural layers (Controller, Service, Repository)
- **N+1 Problem**: A performance anti-pattern where database queries are executed inside loops

## Requirements

### Requirement 1: Import Resolution and FQN Mapping

**User Story:** As a developer, I want the analyzer to correctly resolve class names across packages, so that cross-file analysis (like N+1 detection) works accurately even when different packages have classes with the same simple name.

#### Acceptance Criteria

1. WHEN the analyzer parses a Java file THEN the system SHALL extract all import statements including explicit imports and wildcard imports
2. WHEN a simple class name is encountered in code THEN the system SHALL resolve it to its FQN using the following priority: explicit imports, wildcard imports, same-package classes, java.lang classes
3. WHEN two classes have the same simple name but different packages THEN the system SHALL distinguish them correctly in the SymbolTable
4. WHEN building the CallGraph THEN the system SHALL use FQNs for method signatures instead of simple class names
5. WHEN a wildcard import is used (e.g., `import com.example.*`) THEN the system SHALL attempt to resolve simple names against all classes discovered in that package

### Requirement 2: Enhanced ThreadLocal Leak Detection

**User Story:** As a developer, I want accurate ThreadLocal leak detection, so that I only receive warnings when `remove()` is genuinely missing from the proper cleanup path.

#### Acceptance Criteria

1. WHEN a ThreadLocal.set() call is detected THEN the system SHALL verify whether a corresponding remove() call exists in a finally block within the same method
2. WHEN remove() is called outside a finally block THEN the system SHALL still report a potential leak warning with reduced severity
3. WHEN remove() is called in a finally block for the same ThreadLocal variable THEN the system SHALL NOT report a leak
4. IF the method contains a try-finally structure with remove() in finally THEN the system SHALL mark the ThreadLocal usage as safe

### Requirement 3: Structured Project Detection

**User Story:** As a developer, I want reliable project stack detection, so that the analyzer applies the correct rules based on actual project dependencies rather than false positives from comments or test dependencies.

#### Acceptance Criteria

1. WHEN analyzing pom.xml THEN the system SHALL parse the XML structure to extract dependencies
2. WHEN a dependency is in test scope THEN the system SHALL NOT consider it for main project stack detection
3. WHEN a dependency is commented out in XML THEN the system SHALL NOT consider it as an active dependency
4. WHEN analyzing build.gradle THEN the system SHALL distinguish between implementation, testImplementation, and other configurations
5. WHEN detecting reactive stack THEN the system SHALL only flag it if webflux/reactor is in main (non-test) dependencies

### Requirement 4: CallGraph Type Resolution Integration

**User Story:** As a developer, I want the CallGraph to accurately track method calls through field references, so that N+1 detection can trace calls from Controller through Service to Repository layers.

#### Acceptance Criteria

1. WHEN a method call is made on a field (e.g., `userRepository.findById()`) THEN the system SHALL resolve the field's type using the SymbolTable
2. WHEN the field type is resolved THEN the system SHALL use the resolved FQN to create accurate CallGraph edges
3. WHEN tracing a call chain from Controller to Repository THEN the system SHALL correctly identify the path even when intermediate classes are in different packages
4. WHEN a field type cannot be resolved THEN the system SHALL fall back to heuristic detection with appropriate confidence marking

### Requirement 5: Import Index Data Structure

**User Story:** As a developer, I want the import information to be efficiently stored and queryable, so that FQN resolution is fast during analysis.

#### Acceptance Criteria

1. WHEN parsing imports THEN the system SHALL build an ImportIndex containing explicit import mappings and wildcard import packages
2. WHEN the ImportIndex is queried with a simple name THEN the system SHALL return the FQN in O(1) time for explicit imports
3. WHEN merging ImportIndex from multiple files THEN the system SHALL maintain per-file import scopes without cross-contamination
4. WHEN a class is defined in the current file THEN the system SHALL automatically add it to the ImportIndex with its package prefix
