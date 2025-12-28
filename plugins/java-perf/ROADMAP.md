# Roadmap

Development roadmap for Java Perf plugin. For current version, see `plugin.json`.

## 🚧 Planned Improvements

### 1. Enhanced Call Graph Resolution

**Current Limitation**:
- `extract_call_sites()` captures variable names (e.g., `userRepository`)
- Cannot directly map to fully qualified class names (e.g., `com.example.repository.UserRepository`)

**Proposed Solution**:

```rust
// New structure for import resolution
pub struct ImportIndex {
    /// Simple class name -> FQN (e.g., "UserRepository" -> "com.example.repository.UserRepository")
    simple_to_fqn: HashMap<String, String>,
    /// Wildcard imports (e.g., "com.example.repository.*")
    wildcard_imports: Vec<String>,
}

// Build during Phase 1 indexing
impl JavaTreeSitterAnalyzer {
    pub fn extract_imports_index(&self, code: &str) -> Result<ImportIndex> {
        let imports = self.extract_imports(code)?;
        let mut index = ImportIndex::new();

        for import in imports {
            if import.ends_with(".*") {
                index.wildcard_imports.push(import.trim_end_matches(".*").to_string());
            } else {
                let simple_name = import.rsplit('.').next().unwrap_or(&import);
                index.simple_to_fqn.insert(simple_name.to_string(), import);
            }
        }

        Ok(index)
    }
}
```

**Impact**: Improved N+1 detection accuracy across packages
**Effort**: ~2-3 hours

---

### 2. Enhanced Spring Context Understanding

**Current Limitation**:
- @Autowired field tracking depends on variable and type names
- Cannot handle @Qualifier, @Resource(name="xxx") and other complex cases

**Proposed Solution**:

```rust
// Extended VarBinding
pub struct VarBinding {
    pub name: String,
    pub type_name: String,
    pub is_field: bool,
    pub qualifier: Option<String>,  // @Qualifier("xxx") or @Resource(name="xxx")
}
```

**Impact**: Better handling of edge cases in Spring DI
**Effort**: ~1-2 hours

---

### 3. Structured Configuration Parsing

**Current Limitation**:
- `LineBasedConfigAnalyzer` uses line matching
- serde_yaml introduced but not fully migrated

**Proposed Solution**:

```rust
#[derive(Debug, Deserialize)]
struct SpringConfig {
    spring: Option<SpringSection>,
    server: Option<ServerSection>,
    management: Option<ManagementSection>,
}

#[derive(Debug, Deserialize)]
struct DataSourceConfig {
    url: Option<String>,
    #[serde(rename = "hikari")]
    hikari: Option<HikariConfig>,
}

fn analyze_yaml_structured(content: &str, file: &str) -> Vec<Issue> {
    let config: SpringConfig = serde_yaml::from_str(content)?;
    // Structured detection logic
}
```

**Impact**: More reliable configuration issue detection
**Effort**: ~3-4 hours

---

## Priority Matrix

| Task | Priority | Impact | Effort |
|------|----------|--------|--------|
| Call Graph + Import | High | N+1 detection accuracy | 2-3h |
| Structured Config | Medium | Config issue detection | 3-4h |
| Spring Context | Low | Edge cases | 1-2h |

---

## Testing Strategy

Each improvement requires:
1. Unit tests covering core logic
2. Integration tests for end-to-end validation
3. Verification with real Spring Boot projects
