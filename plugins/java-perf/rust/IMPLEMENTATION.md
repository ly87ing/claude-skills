# Java Perf - Rust Implementation

Technical implementation guide for the Java Perf CLI tool.

## 🚀 Performance Advantages

| Metric | Node.js (v3.x) | Rust |
|--------|---------------|------|
| Dependencies | Node.js + npm install | **Zero** |
| Binary Size | ~50MB | **1.9MB** |
| Startup Time | ~500ms | **~5ms** |
| Memory Usage | ~50MB | **~5MB** |

## 📦 Building from Source

### Prerequisites

- Rust toolchain (rustup recommended)
- Cargo package manager

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Install to local bin
cp target/release/java-perf ~/.local/bin/
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

## 🏗️ Architecture

```
src/
├── main.rs              # CLI entry point
├── cli.rs               # Command line argument parsing (clap)
├── ast_engine.rs        # Tree-sitter Java AST analysis
├── checklist.rs         # Checklist and anti-pattern knowledge base
├── forensic.rs          # Log fingerprint classification (streaming)
├── jdk_engine.rs        # JDK CLI wrappers (jstack/javap/jmap)
├── project_detector.rs  # Project type detection (Spring Boot/WebFlux)
├── symbol_table.rs      # Cross-file symbol resolution
├── taint.rs             # Taint analysis for call graph
├── scanner/             # Scanner module
│   ├── mod.rs           # Scanner orchestration
│   ├── config.rs        # Configuration parsing
│   ├── dockerfile.rs    # Dockerfile analysis
│   ├── queries.rs       # Tree-sitter query management
│   ├── rule_handlers.rs # Rule handler implementations
│   └── tree_sitter_java.rs # Java-specific AST utilities
└── rules/               # Rule definitions
    ├── mod.rs           # Rule module exports
    ├── definitions.rs   # Rule metadata and severity
    └── suppression.rs   # Suppression comment handling
```

## 🔍 Detection Engine

### Tree-sitter AST Analysis

The core detection engine uses Tree-sitter for parsing Java source code into AST:

```rust
// Example: N+1 detection query
(for_statement
  body: (block
    (expression_statement
      (method_invocation
        name: (identifier) @method))))
```

### Two-Pass Architecture

1. **Phase 1 - Indexing**: Build symbol table (classes, fields, annotations)
2. **Phase 2 - Analysis**: Context-aware rule evaluation with symbol resolution

### Rule Handlers

Rules are implemented as trait objects for polymorphic dispatch:

```rust
pub trait RuleHandler: Send + Sync {
    fn rule_id(&self) -> &'static str;
    fn check(&self, ctx: &RuleContext) -> Vec<Finding>;
}
```

## 📁 Resources

### Query Files

Tree-sitter queries are externalized in `resources/queries/`:

- `n_plus_one.scm` - N+1 detection patterns
- `concurrency.scm` - Concurrency issue patterns
- `sql_issues.scm` - SQL anti-pattern detection

### Test Fixtures

Sample Java projects for testing in `fixtures/`:

- `spring-boot-sample/` - Spring Boot application patterns

## 🔧 Configuration

The scanner supports configuration via:

- Command line arguments
- Project detection (auto-configures based on detected framework)
- Suppression comments in source code

## License

MIT
