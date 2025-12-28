// ============================================================================
// 项目侦测模块 - 识别技术栈与版本
// ============================================================================

use std::path::Path;
use std::fs;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

// ============================================================================
// Maven Dependency Types (Requirements 3.1)
// ============================================================================

/// Dependency scope in Maven pom.xml
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DependencyScope {
    #[default]
    Compile,
    Test,
    Runtime,
    Provided,
    System,
    Import,
}

impl FromStr for DependencyScope {
    type Err = std::convert::Infallible;
    
    /// Parse scope string from pom.xml
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "test" => DependencyScope::Test,
            "runtime" => DependencyScope::Runtime,
            "provided" => DependencyScope::Provided,
            "system" => DependencyScope::System,
            "import" => DependencyScope::Import,
            _ => DependencyScope::Compile, // Default scope is compile
        })
    }
}

impl DependencyScope {
    /// Check if this scope is for main (non-test) dependencies
    pub fn is_main_scope(&self) -> bool {
        !matches!(self, DependencyScope::Test)
    }
}

/// Parsed Maven dependency from pom.xml
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenDependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: DependencyScope,
}

// Test-only builder methods
#[cfg(test)]
#[allow(dead_code)]
impl MavenDependency {
    pub fn new(group_id: &str, artifact_id: &str) -> Self {
        MavenDependency {
            group_id: group_id.to_string(),
            artifact_id: artifact_id.to_string(),
            version: None,
            scope: DependencyScope::default(),
        }
    }
    
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }
    
    pub fn with_scope(mut self, scope: DependencyScope) -> Self {
        self.scope = scope;
        self
    }
    
    /// Get the full coordinate string (groupId:artifactId)
    pub fn coordinate(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

// ============================================================================
// Maven POM Parsing (Requirements 3.1, 3.3)
// ============================================================================

use quick_xml::events::Event;
use quick_xml::Reader;

/// Parse Maven pom.xml content and extract dependencies
/// 
/// This function:
/// - Extracts groupId, artifactId, version, and scope from each dependency
/// - Properly handles XML comments (skips commented elements)
/// - Returns only actual dependencies, not commented-out ones
/// 
/// # Arguments
/// * `content` - The raw XML content of pom.xml
/// 
/// # Returns
/// * `Result<Vec<MavenDependency>>` - List of parsed dependencies or error
pub fn parse_maven_pom(content: &str) -> Result<Vec<MavenDependency>, String> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    
    let mut dependencies = Vec::new();
    let mut buf = Vec::new();
    
    // State tracking
    let mut in_dependencies = false;
    let mut in_dependency = false;
    let mut in_dependency_management = false;
    let mut current_dep: Option<PartialDependency> = None;
    let mut current_element: Option<String> = None;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                match name.as_str() {
                    "dependencyManagement" => {
                        in_dependency_management = true;
                    }
                    "dependencies" if !in_dependency_management => {
                        in_dependencies = true;
                    }
                    "dependency" if in_dependencies && !in_dependency_management => {
                        in_dependency = true;
                        current_dep = Some(PartialDependency::default());
                    }
                    "groupId" | "artifactId" | "version" | "scope" if in_dependency => {
                        current_element = Some(name);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                match name.as_str() {
                    "dependencyManagement" => {
                        in_dependency_management = false;
                    }
                    "dependencies" if !in_dependency_management => {
                        in_dependencies = false;
                    }
                    "dependency" if in_dependency && !in_dependency_management => {
                        in_dependency = false;
                        if let Some(partial) = current_dep.take() {
                            if let Some(dep) = partial.into_dependency() {
                                dependencies.push(dep);
                            }
                        }
                    }
                    "groupId" | "artifactId" | "version" | "scope" => {
                        current_element = None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if let Some(ref element) = current_element {
                    if let Some(ref mut dep) = current_dep {
                        let text = e.unescape().map_err(|e| e.to_string())?.to_string();
                        match element.as_str() {
                            "groupId" => dep.group_id = Some(text),
                            "artifactId" => dep.artifact_id = Some(text),
                            "version" => dep.version = Some(text),
                            "scope" => dep.scope = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Comment(_)) => {
                // XML comments are automatically skipped by quick-xml
                // No action needed - commented dependencies won't be parsed
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }
    
    Ok(dependencies)
}

/// Helper struct for building MavenDependency during parsing
#[derive(Default)]
struct PartialDependency {
    group_id: Option<String>,
    artifact_id: Option<String>,
    version: Option<String>,
    scope: Option<String>,
}

impl PartialDependency {
    fn into_dependency(self) -> Option<MavenDependency> {
        let group_id = self.group_id?;
        let artifact_id = self.artifact_id?;
        
        let scope = self.scope
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
        
        Some(MavenDependency {
            group_id,
            artifact_id,
            version: self.version,
            scope,
        })
    }
}

/// Filter dependencies to only include main (non-test) scope
pub fn filter_main_dependencies(deps: &[MavenDependency]) -> Vec<&MavenDependency> {
    deps.iter().filter(|d| d.scope.is_main_scope()).collect()
}

// ============================================================================
// Gradle Dependency Types (Requirements 3.4)
// ============================================================================

/// Gradle dependency configuration type
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GradleConfiguration {
    #[default]
    Implementation,
    TestImplementation,
    CompileOnly,
    RuntimeOnly,
    TestCompileOnly,
    TestRuntimeOnly,
    Api,
    AnnotationProcessor,
    Other(String),
}

impl FromStr for GradleConfiguration {
    type Err = std::convert::Infallible;
    
    /// Parse configuration string from build.gradle
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "implementation" => GradleConfiguration::Implementation,
            "testImplementation" => GradleConfiguration::TestImplementation,
            "compileOnly" => GradleConfiguration::CompileOnly,
            "runtimeOnly" => GradleConfiguration::RuntimeOnly,
            "testCompileOnly" => GradleConfiguration::TestCompileOnly,
            "testRuntimeOnly" => GradleConfiguration::TestRuntimeOnly,
            "api" => GradleConfiguration::Api,
            "annotationProcessor" => GradleConfiguration::AnnotationProcessor,
            other => GradleConfiguration::Other(other.to_string()),
        })
    }
}

impl GradleConfiguration {
    /// Check if this configuration is for main (non-test) dependencies
    pub fn is_main_configuration(&self) -> bool {
        !matches!(
            self,
            GradleConfiguration::TestImplementation
                | GradleConfiguration::TestCompileOnly
                | GradleConfiguration::TestRuntimeOnly
        )
    }
}

/// Parsed Gradle dependency from build.gradle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradleDependency {
    pub group: String,
    pub name: String,
    pub version: Option<String>,
    pub configuration: GradleConfiguration,
}

// Test-only builder methods
#[cfg(test)]
#[allow(dead_code)]
impl GradleDependency {
    pub fn new(group: &str, name: &str, configuration: GradleConfiguration) -> Self {
        GradleDependency {
            group: group.to_string(),
            name: name.to_string(),
            version: None,
            configuration,
        }
    }
    
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }
    
    /// Get the full coordinate string (group:name)
    pub fn coordinate(&self) -> String {
        format!("{}:{}", self.group, self.name)
    }
}

/// Detect stack from parsed Maven dependencies (excluding test scope)
/// Requirements 3.2, 3.5
pub fn detect_stack_from_maven_deps(deps: &[MavenDependency]) -> DetectedStack {
    let mut stack = DetectedStack {
        is_maven: true,
        build_tool: "maven".to_string(),
        ..Default::default()
    };
    
    // Only consider main (non-test) dependencies
    let main_deps = filter_main_dependencies(deps);
    
    for dep in main_deps {
        let artifact = dep.artifact_id.as_str();
        let group = dep.group_id.as_str();
        
        // Spring Boot detection
        if artifact.contains("spring-boot-starter") {
            stack.is_spring_boot = true;
        }
        
        // Spring MVC detection
        if artifact == "spring-boot-starter-web" {
            stack.is_spring_mvc = true;
        }
        
        // Reactive stack detection (Requirements 3.5)
        if artifact == "spring-boot-starter-webflux" 
            || artifact == "reactor-core"
            || (group == "io.projectreactor" && artifact.starts_with("reactor-")) {
            stack.is_reactive = true;
        }
        
        // Lombok detection
        if artifact == "lombok" || group == "org.projectlombok" {
            stack.has_lombok = true;
        }
    }
    
    stack
}

// ============================================================================
// Gradle Build File Parsing (Requirements 3.4)
// ============================================================================

use regex::Regex;

/// Parse Gradle build.gradle content and extract dependencies
/// 
/// This function:
/// - Extracts group, name, version from each dependency declaration
/// - Distinguishes between implementation, testImplementation, and other configurations
/// - Handles both string notation ("group:name:version") and map notation
/// - Handles single-line comments (// ...) by ignoring commented lines
/// 
/// # Arguments
/// * `content` - The raw content of build.gradle or build.gradle.kts
/// 
/// # Returns
/// * `Result<Vec<GradleDependency>>` - List of parsed dependencies or error
pub fn parse_gradle_build(content: &str) -> Result<Vec<GradleDependency>, String> {
    let mut dependencies = Vec::new();
    
    // Regex for string notation: configuration "group:name:version" or configuration 'group:name:version'
    // Also handles configuration("group:name:version") for Kotlin DSL
    let string_notation = Regex::new(
        r#"(?m)^\s*(implementation|testImplementation|compileOnly|runtimeOnly|testCompileOnly|testRuntimeOnly|api|annotationProcessor)\s*[\(]?\s*["']([^"']+)["']\s*[\)]?"#
    ).map_err(|e| format!("Regex error: {}", e))?;
    
    // Regex for platform/BOM notation: configuration platform("group:name:version")
    let platform_notation = Regex::new(
        r#"(?m)^\s*(implementation|testImplementation|compileOnly|runtimeOnly|testCompileOnly|testRuntimeOnly|api)\s*[\(]?\s*platform\s*\(\s*["']([^"']+)["']\s*\)\s*[\)]?"#
    ).map_err(|e| format!("Regex error: {}", e))?;
    
    // Process each line, skipping comments
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Skip single-line comments
        if trimmed.starts_with("//") {
            continue;
        }
        
        // Remove inline comments
        let line_without_comment = if let Some(idx) = trimmed.find("//") {
            &trimmed[..idx]
        } else {
            trimmed
        };
        
        // Try platform notation first (more specific)
        if let Some(caps) = platform_notation.captures(line_without_comment) {
            let config_str = caps.get(1).map(|m| m.as_str()).unwrap_or("implementation");
            let dep_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            if let Some(dep) = parse_dependency_string(dep_str, config_str) {
                dependencies.push(dep);
            }
            continue;
        }
        
        // Try string notation
        if let Some(caps) = string_notation.captures(line_without_comment) {
            let config_str = caps.get(1).map(|m| m.as_str()).unwrap_or("implementation");
            let dep_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            if let Some(dep) = parse_dependency_string(dep_str, config_str) {
                dependencies.push(dep);
            }
        }
    }
    
    Ok(dependencies)
}

/// Parse a dependency string in format "group:name:version" or "group:name"
fn parse_dependency_string(dep_str: &str, config_str: &str) -> Option<GradleDependency> {
    let parts: Vec<&str> = dep_str.split(':').collect();
    
    if parts.len() >= 2 {
        let group = parts[0].to_string();
        let name = parts[1].to_string();
        let version = if parts.len() >= 3 {
            Some(parts[2].to_string())
        } else {
            None
        };
        let configuration = config_str.parse().unwrap_or_default();
        
        Some(GradleDependency {
            group,
            name,
            version,
            configuration,
        })
    } else {
        None
    }
}

/// Filter Gradle dependencies to only include main (non-test) configurations
pub fn filter_main_gradle_dependencies(deps: &[GradleDependency]) -> Vec<&GradleDependency> {
    deps.iter().filter(|d| d.configuration.is_main_configuration()).collect()
}

/// Detect stack from parsed Gradle dependencies (excluding test configurations)
/// Requirements 3.4
pub fn detect_stack_from_gradle_deps(deps: &[GradleDependency]) -> DetectedStack {
    let mut stack = DetectedStack {
        is_gradle: true,
        build_tool: "gradle".to_string(),
        ..Default::default()
    };
    
    // Only consider main (non-test) dependencies
    let main_deps = filter_main_gradle_dependencies(deps);
    
    for dep in main_deps {
        let name = dep.name.as_str();
        let group = dep.group.as_str();
        
        // Spring Boot detection
        if name.contains("spring-boot-starter") || group == "org.springframework.boot" {
            stack.is_spring_boot = true;
        }
        
        // Spring MVC detection
        if name == "spring-boot-starter-web" {
            stack.is_spring_mvc = true;
        }
        
        // Reactive stack detection
        if name == "spring-boot-starter-webflux" 
            || name == "reactor-core"
            || (group == "io.projectreactor" && name.starts_with("reactor-")) {
            stack.is_reactive = true;
        }
        
        // Lombok detection
        if name == "lombok" || group == "org.projectlombok" {
            stack.has_lombok = true;
        }
    }
    
    stack
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DetectedStack {
    pub is_spring_boot: bool,
    pub is_spring_mvc: bool,
    pub is_reactive: bool,      // WebFlux, Vert.x, Reactor
    pub is_maven: bool,
    pub is_gradle: bool,
    pub has_lombok: bool,
    pub jdk_version: String,    // "8", "11", "17", "21"
    pub build_tool: String,     // "maven" or "gradle"
}


/// 扫描项目目录，检测技术栈
pub fn detect_stack(root: &Path) -> DetectedStack {
    let mut stack = DetectedStack::default();
    
    // 1. 检测构建工具
    if root.join("pom.xml").exists() {
        stack.is_maven = true;
        stack.build_tool = "maven".to_string();
        analyze_maven(root, &mut stack);
    } else if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
        stack.is_gradle = true;
        stack.build_tool = "gradle".to_string();
        analyze_gradle(root, &mut stack);
    }
    
    // 2. 默认值兜底
    if stack.jdk_version.is_empty() {
        stack.jdk_version = "1.8".to_string(); // 默认假设
    }
    
    stack
}

fn analyze_maven(root: &Path, stack: &mut DetectedStack) {
    if let Ok(content) = fs::read_to_string(root.join("pom.xml")) {
        // Use structured XML parsing to extract dependencies (Requirements 3.1, 3.2)
        // This properly filters out test-scoped and commented dependencies
        match parse_maven_pom(&content) {
            Ok(deps) => {
                // Detect stack from parsed dependencies (excludes test scope)
                let detected = detect_stack_from_maven_deps(&deps);
                stack.is_spring_boot = detected.is_spring_boot;
                stack.is_spring_mvc = detected.is_spring_mvc;
                stack.is_reactive = detected.is_reactive;
                stack.has_lombok = detected.has_lombok;
            }
            Err(_) => {
                // Fall back to simple string matching if XML parsing fails
                // This provides resilience for malformed pom.xml files
                if content.contains("spring-boot-starter") {
                    stack.is_spring_boot = true;
                }
                if content.contains("spring-boot-starter-web") {
                    stack.is_spring_mvc = true;
                }
                if content.contains("spring-boot-starter-webflux") || content.contains("reactor-core") {
                    stack.is_reactive = true;
                }
                if content.contains("lombok") {
                    stack.has_lombok = true;
                }
            }
        }
        
        // Extract JDK version from properties (still use string matching as it's in <properties>)
        if content.contains("<java.version>17") || content.contains("<target>17") {
            stack.jdk_version = "17".to_string();
        } else if content.contains("<java.version>21") || content.contains("<target>21") {
            stack.jdk_version = "21".to_string();
        } else if content.contains("<java.version>11") || content.contains("<target>11") {
            stack.jdk_version = "11".to_string();
        }
    }
}

fn analyze_gradle(root: &Path, stack: &mut DetectedStack) {
    let gradle_files = ["build.gradle", "build.gradle.kts"];
    for file in gradle_files {
        if let Ok(content) = fs::read_to_string(root.join(file)) {
            // Use structured parsing to extract dependencies (Requirements 3.4)
            // This properly filters out testImplementation and other test configurations
            match parse_gradle_build(&content) {
                Ok(deps) => {
                    // Detect stack from parsed dependencies (excludes test configurations)
                    let detected = detect_stack_from_gradle_deps(&deps);
                    stack.is_spring_boot = stack.is_spring_boot || detected.is_spring_boot;
                    stack.is_spring_mvc = stack.is_spring_mvc || detected.is_spring_mvc;
                    stack.is_reactive = stack.is_reactive || detected.is_reactive;
                    stack.has_lombok = stack.has_lombok || detected.has_lombok;
                }
                Err(_) => {
                    // Fall back to simple string matching if parsing fails
                    // This provides resilience for complex Gradle DSL constructs
                    if content.contains("org.springframework.boot") || content.contains("spring-boot-starter") {
                        stack.is_spring_boot = true;
                    }
                    if content.contains("webflux") || content.contains("reactor") {
                        stack.is_reactive = true;
                    }
                }
            }
            
            // Extract JDK version (still use string matching as it's in build config)
            if content.contains("JavaVersion.VERSION_17") || content.contains("sourceCompatibility = '17'") {
                stack.jdk_version = "17".to_string();
            } else if content.contains("JavaVersion.VERSION_21") || content.contains("sourceCompatibility = '21'") {
                stack.jdk_version = "21".to_string();
            }
        }
    }
}

/// 根据检测到的技术栈生成分析指导策略
pub fn generate_strategy_hint(stack: &DetectedStack) -> String {
    let mut hints = Vec::new();
    
    hints.push(format!("Project Type: {} (JDK {})", 
        if stack.is_spring_boot { "Spring Boot" } else { "Java Application" }, 
        stack.jdk_version
    ));
    
    if stack.jdk_version == "21" {
        hints.push("- **Virtual Threads**: Check for `synchronized` pinning. Suggest `ReentrantLock`.".to_string());
    }
    
    if stack.is_spring_boot {
        hints.push("- **Spring Boot**: Focus on Bean lifecycle, Dependency Injection, and Auto-configuration.".to_string());
        if stack.is_reactive {
            hints.push("- **Reactive (WebFlux)**: Focus on Backpressure, blocking calls in Reactor threads (`.block()`). Ignore ThreadLocal issues.".to_string());
        } else {
            hints.push("- **Servlet (MVC)**: Focus on Thread Pool exhaustion, blocking I/O headers.".to_string());
        }
    }
    
    if stack.has_lombok {
        hints.push("- **Lombok**: Be aware of generated code (equals/hashCode) performance impacts.".to_string());
    }
    
    hints.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    // ========================================================================
    // Test-only types for unified dependency detection
    // ========================================================================
    
    /// A generic dependency representation for unified stack detection (test-only)
    #[derive(Debug, Clone)]
    pub struct GenericDependency {
        pub group: String,
        pub artifact: String,
        pub is_test_scope: bool,
    }

    impl From<&MavenDependency> for GenericDependency {
        fn from(dep: &MavenDependency) -> Self {
            GenericDependency {
                group: dep.group_id.clone(),
                artifact: dep.artifact_id.clone(),
                is_test_scope: dep.scope == DependencyScope::Test,
            }
        }
    }

    impl From<&GradleDependency> for GenericDependency {
        fn from(dep: &GradleDependency) -> Self {
            GenericDependency {
                group: dep.group.clone(),
                artifact: dep.name.clone(),
                is_test_scope: !dep.configuration.is_main_configuration(),
            }
        }
    }

    /// Detect stack from a filtered list of generic dependencies (test-only)
    pub fn detect_stack_from_deps(deps: &[GenericDependency]) -> DetectedStack {
        let mut stack = DetectedStack::default();
        
        let main_deps: Vec<_> = deps.iter()
            .filter(|d| !d.is_test_scope)
            .collect();
        
        for dep in main_deps {
            let artifact = dep.artifact.as_str();
            let group = dep.group.as_str();
            
            if artifact.contains("spring-boot-starter") || group == "org.springframework.boot" {
                stack.is_spring_boot = true;
            }
            if artifact == "spring-boot-starter-web" {
                stack.is_spring_mvc = true;
            }
            if artifact == "spring-boot-starter-webflux" 
                || artifact == "reactor-core"
                || (group == "io.projectreactor" && artifact.starts_with("reactor-")) {
                stack.is_reactive = true;
            }
            if artifact == "lombok" || group == "org.projectlombok" {
                stack.has_lombok = true;
            }
        }
        
        stack
    }

    /// Convert Maven dependencies to generic dependencies and detect stack (test-only)
    pub fn detect_stack_from_maven_deps_generic(deps: &[MavenDependency]) -> DetectedStack {
        let generic_deps: Vec<GenericDependency> = deps.iter().map(|d| d.into()).collect();
        let mut stack = detect_stack_from_deps(&generic_deps);
        stack.is_maven = true;
        stack.build_tool = "maven".to_string();
        stack
    }

    /// Convert Gradle dependencies to generic dependencies and detect stack (test-only)
    pub fn detect_stack_from_gradle_deps_generic(deps: &[GradleDependency]) -> DetectedStack {
        let generic_deps: Vec<GenericDependency> = deps.iter().map(|d| d.into()).collect();
        let mut stack = detect_stack_from_deps(&generic_deps);
        stack.is_gradle = true;
        stack.build_tool = "gradle".to_string();
        stack
    }

    #[test]
    fn test_detect_spring_boot_maven() {
        let dir = tempdir().unwrap();
        let pom = dir.path().join("pom.xml");
        let mut file = File::create(pom).unwrap();
        writeln!(file, r#"
            <dependencies>
                <dependency>
                    <groupId>org.springframework.boot</groupId>
                    <artifactId>spring-boot-starter-web</artifactId>
                </dependency>
            </dependencies>
            <properties>
                <java.version>17</java.version>
            </properties>
        "#).unwrap();
        
        let stack = detect_stack(dir.path());
        assert!(stack.is_spring_boot);
        assert!(stack.is_spring_mvc);
        assert_eq!(stack.build_tool, "maven");
        assert_eq!(stack.jdk_version, "17");
        
        let hint = generate_strategy_hint(&stack);
        assert!(hint.contains("Spring Boot"));
        assert!(hint.contains("JDK 17"));
    }
    
    // ========================================================================
    // Unit tests for parse_maven_pom
    // ========================================================================
    
    #[test]
    fn test_parse_maven_pom_basic() {
        let pom = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
            <version>3.2.0</version>
        </dependency>
    </dependencies>
</project>"#;
        
        let deps = parse_maven_pom(pom).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].group_id, "org.springframework.boot");
        assert_eq!(deps[0].artifact_id, "spring-boot-starter-web");
        assert_eq!(deps[0].version, Some("3.2.0".to_string()));
        assert_eq!(deps[0].scope, DependencyScope::Compile);
    }
    
    #[test]
    fn test_parse_maven_pom_with_test_scope() {
        let pom = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-web</artifactId>
        </dependency>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter-test</artifactId>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#;
        
        let deps = parse_maven_pom(pom).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].scope, DependencyScope::Compile);
        assert_eq!(deps[1].scope, DependencyScope::Test);
    }
    
    #[test]
    fn test_detect_stack_excludes_test_scope() {
        let pom = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <dependencies>
        <dependency>
            <groupId>io.projectreactor</groupId>
            <artifactId>reactor-core</artifactId>
            <scope>test</scope>
        </dependency>
    </dependencies>
</project>"#;
        
        let deps = parse_maven_pom(pom).unwrap();
        let stack = detect_stack_from_maven_deps(&deps);
        
        // reactor-core is test-scoped, so is_reactive should be false
        assert!(!stack.is_reactive);
    }
    
    // ========================================================================
    // Unit tests for parse_gradle_build
    // ========================================================================
    
    #[test]
    fn test_parse_gradle_build_basic() {
        let gradle = r#"
plugins {
    id 'java'
}

dependencies {
    implementation 'org.springframework.boot:spring-boot-starter-web:3.2.0'
    testImplementation 'org.junit.jupiter:junit-jupiter:5.10.0'
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].group, "org.springframework.boot");
        assert_eq!(deps[0].name, "spring-boot-starter-web");
        assert_eq!(deps[0].version, Some("3.2.0".to_string()));
        assert_eq!(deps[0].configuration, GradleConfiguration::Implementation);
        
        assert_eq!(deps[1].group, "org.junit.jupiter");
        assert_eq!(deps[1].name, "junit-jupiter");
        assert_eq!(deps[1].configuration, GradleConfiguration::TestImplementation);
    }
    
    #[test]
    fn test_parse_gradle_build_kotlin_dsl() {
        let gradle = r#"
plugins {
    kotlin("jvm")
}

dependencies {
    implementation("org.springframework.boot:spring-boot-starter-web:3.2.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].group, "org.springframework.boot");
        assert_eq!(deps[0].name, "spring-boot-starter-web");
        assert_eq!(deps[0].configuration, GradleConfiguration::Implementation);
        
        assert_eq!(deps[1].configuration, GradleConfiguration::TestImplementation);
    }
    
    #[test]
    fn test_parse_gradle_build_skips_comments() {
        let gradle = r#"
dependencies {
    implementation 'org.springframework.boot:spring-boot-starter-web:3.2.0'
    // testImplementation 'org.junit.jupiter:junit-jupiter:5.10.0'
    // implementation 'commented:out:1.0.0'
    compileOnly 'org.projectlombok:lombok:1.18.30'
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "spring-boot-starter-web");
        assert_eq!(deps[1].name, "lombok");
    }
    
    #[test]
    fn test_parse_gradle_build_various_configurations() {
        let gradle = r#"
dependencies {
    implementation 'com.example:impl:1.0'
    api 'com.example:api:1.0'
    compileOnly 'com.example:compile-only:1.0'
    runtimeOnly 'com.example:runtime-only:1.0'
    testImplementation 'com.example:test-impl:1.0'
    testCompileOnly 'com.example:test-compile:1.0'
    testRuntimeOnly 'com.example:test-runtime:1.0'
    annotationProcessor 'com.example:processor:1.0'
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        assert_eq!(deps.len(), 8);
        
        assert_eq!(deps[0].configuration, GradleConfiguration::Implementation);
        assert_eq!(deps[1].configuration, GradleConfiguration::Api);
        assert_eq!(deps[2].configuration, GradleConfiguration::CompileOnly);
        assert_eq!(deps[3].configuration, GradleConfiguration::RuntimeOnly);
        assert_eq!(deps[4].configuration, GradleConfiguration::TestImplementation);
        assert_eq!(deps[5].configuration, GradleConfiguration::TestCompileOnly);
        assert_eq!(deps[6].configuration, GradleConfiguration::TestRuntimeOnly);
        assert_eq!(deps[7].configuration, GradleConfiguration::AnnotationProcessor);
    }
    
    #[test]
    fn test_detect_stack_from_gradle_excludes_test() {
        let gradle = r#"
dependencies {
    testImplementation 'io.projectreactor:reactor-core:3.6.0'
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        let stack = detect_stack_from_gradle_deps(&deps);
        
        // reactor-core is testImplementation, so is_reactive should be false
        assert!(!stack.is_reactive);
    }
    
    #[test]
    fn test_detect_stack_from_gradle_includes_main() {
        let gradle = r#"
dependencies {
    implementation 'org.springframework.boot:spring-boot-starter-webflux:3.2.0'
}
"#;
        
        let deps = parse_gradle_build(gradle).unwrap();
        let stack = detect_stack_from_gradle_deps(&deps);
        
        // webflux is implementation, so is_reactive should be true
        assert!(stack.is_reactive);
        assert!(stack.is_spring_boot);
    }
    
    // ========================================================================
    // Unit tests for detect_stack_from_deps (unified function)
    // ========================================================================
    
    #[test]
    fn test_detect_stack_from_deps_filters_test_scope() {
        // Create generic dependencies with mixed scopes
        let deps = vec![
            GenericDependency {
                group: "org.springframework.boot".to_string(),
                artifact: "spring-boot-starter-webflux".to_string(),
                is_test_scope: true, // Test scope - should be filtered
            },
            GenericDependency {
                group: "org.springframework.boot".to_string(),
                artifact: "spring-boot-starter-web".to_string(),
                is_test_scope: false, // Main scope - should be included
            },
        ];
        
        let stack = detect_stack_from_deps(&deps);
        
        // webflux is test-scoped, so is_reactive should be false
        assert!(!stack.is_reactive);
        // spring-boot-starter-web is main scope, so is_spring_boot and is_spring_mvc should be true
        assert!(stack.is_spring_boot);
        assert!(stack.is_spring_mvc);
    }
    
    #[test]
    fn test_detect_stack_from_deps_detects_reactive() {
        let deps = vec![
            GenericDependency {
                group: "io.projectreactor".to_string(),
                artifact: "reactor-core".to_string(),
                is_test_scope: false,
            },
        ];
        
        let stack = detect_stack_from_deps(&deps);
        assert!(stack.is_reactive);
    }
    
    #[test]
    fn test_detect_stack_from_deps_detects_lombok() {
        let deps = vec![
            GenericDependency {
                group: "org.projectlombok".to_string(),
                artifact: "lombok".to_string(),
                is_test_scope: false,
            },
        ];
        
        let stack = detect_stack_from_deps(&deps);
        assert!(stack.has_lombok);
    }
    
    #[test]
    fn test_detect_stack_from_maven_deps_generic() {
        let deps = vec![
            MavenDependency {
                group_id: "org.springframework.boot".to_string(),
                artifact_id: "spring-boot-starter-web".to_string(),
                version: Some("3.2.0".to_string()),
                scope: DependencyScope::Compile,
            },
        ];
        
        let stack = detect_stack_from_maven_deps_generic(&deps);
        assert!(stack.is_spring_boot);
        assert!(stack.is_spring_mvc);
        assert!(stack.is_maven);
        assert_eq!(stack.build_tool, "maven");
    }
    
    #[test]
    fn test_detect_stack_from_gradle_deps_generic() {
        let deps = vec![
            GradleDependency {
                group: "org.springframework.boot".to_string(),
                name: "spring-boot-starter-webflux".to_string(),
                version: Some("3.2.0".to_string()),
                configuration: GradleConfiguration::Implementation,
            },
        ];
        
        let stack = detect_stack_from_gradle_deps_generic(&deps);
        assert!(stack.is_spring_boot);
        assert!(stack.is_reactive);
        assert!(stack.is_gradle);
        assert_eq!(stack.build_tool, "gradle");
    }
}

// ============================================================================
// Property-Based Tests for Maven Parsing
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    /// Generate a valid Maven artifact ID (alphanumeric with hyphens)
    fn arb_artifact_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{2,20}".prop_filter("must not end with hyphen", |s| !s.ends_with('-'))
    }
    
    /// Generate a valid Maven group ID (dot-separated identifiers)
    fn arb_group_id() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-z][a-z0-9]{1,10}", 2..=4)
            .prop_map(|parts| parts.join("."))
    }
    
    /// Generate a valid Maven version string
    fn arb_version() -> impl Strategy<Value = String> {
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
    }
    
    /// Generate a dependency scope
    fn arb_scope() -> impl Strategy<Value = DependencyScope> {
        prop_oneof![
            Just(DependencyScope::Compile),
            Just(DependencyScope::Test),
            Just(DependencyScope::Runtime),
            Just(DependencyScope::Provided),
        ]
    }
    
    /// Generate a MavenDependency
    fn arb_maven_dependency() -> impl Strategy<Value = MavenDependency> {
        (arb_group_id(), arb_artifact_id(), prop::option::of(arb_version()), arb_scope())
            .prop_map(|(group_id, artifact_id, version, scope)| {
                MavenDependency {
                    group_id,
                    artifact_id,
                    version,
                    scope,
                }
            })
    }
    
    /// Generate a list of dependencies
    fn arb_dependencies() -> impl Strategy<Value = Vec<MavenDependency>> {
        prop::collection::vec(arb_maven_dependency(), 0..10)
    }
    
    /// Convert a MavenDependency to XML string
    fn dep_to_xml(dep: &MavenDependency) -> String {
        let mut xml = String::new();
        xml.push_str("        <dependency>\n");
        xml.push_str(&format!("            <groupId>{}</groupId>\n", dep.group_id));
        xml.push_str(&format!("            <artifactId>{}</artifactId>\n", dep.artifact_id));
        if let Some(ref v) = dep.version {
            xml.push_str(&format!("            <version>{}</version>\n", v));
        }
        if dep.scope != DependencyScope::Compile {
            let scope_str = match dep.scope {
                DependencyScope::Test => "test",
                DependencyScope::Runtime => "runtime",
                DependencyScope::Provided => "provided",
                DependencyScope::System => "system",
                DependencyScope::Import => "import",
                DependencyScope::Compile => "compile",
            };
            xml.push_str(&format!("            <scope>{}</scope>\n", scope_str));
        }
        xml.push_str("        </dependency>\n");
        xml
    }
    
    /// Convert a list of dependencies to a complete pom.xml
    fn deps_to_pom(deps: &[MavenDependency]) -> String {
        let mut pom = String::new();
        pom.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <dependencies>
"#);
        for dep in deps {
            pom.push_str(&dep_to_xml(dep));
        }
        pom.push_str("    </dependencies>\n</project>");
        pom
    }
    
    // ========================================================================
    // Property 7: Maven Scope Filtering
    // **Feature: java-perf-semantic-analysis, Property 7: Maven Scope Filtering**
    // **Validates: Requirements 3.1, 3.2, 3.5**
    // 
    // For any pom.xml with test-scoped dependencies, those dependencies SHALL NOT
    // affect the detected project stack (is_spring_boot, is_reactive, etc.).
    // ========================================================================
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_maven_scope_filtering(deps in arb_dependencies()) {
            // Generate pom.xml from dependencies
            let pom_xml = deps_to_pom(&deps);
            
            // Parse the pom.xml
            let parsed_deps = parse_maven_pom(&pom_xml).expect("Should parse valid pom.xml");
            
            // Verify all dependencies were parsed correctly
            prop_assert_eq!(parsed_deps.len(), deps.len(), 
                "Should parse same number of dependencies");
            
            // Detect stack from parsed dependencies
            let stack = detect_stack_from_maven_deps(&parsed_deps);
            
            // Get only main (non-test) dependencies
            let main_deps: Vec<_> = deps.iter()
                .filter(|d| d.scope.is_main_scope())
                .collect();
            
            // Check that test-scoped reactive dependencies don't affect is_reactive
            let has_main_reactive = main_deps.iter().any(|d| {
                d.artifact_id == "spring-boot-starter-webflux" 
                    || d.artifact_id == "reactor-core"
                    || (d.group_id == "io.projectreactor" && d.artifact_id.starts_with("reactor-"))
            });
            
            // If no main-scope reactive deps exist, is_reactive should be false
            if !has_main_reactive {
                prop_assert!(!stack.is_reactive, 
                    "is_reactive should be false when reactive deps are only in test scope");
            }
            
            // Check that test-scoped spring-boot dependencies don't affect is_spring_boot
            let has_main_spring_boot = main_deps.iter().any(|d| {
                d.artifact_id.contains("spring-boot-starter")
            });
            
            if !has_main_spring_boot {
                prop_assert!(!stack.is_spring_boot,
                    "is_spring_boot should be false when spring-boot deps are only in test scope");
            }
        }
    }
    
    // ========================================================================
    // Property 8: XML Comment Handling
    // **Feature: java-perf-semantic-analysis, Property 8: XML Comment Handling**
    // **Validates: Requirements 3.3**
    // 
    // For any dependency element inside an XML comment in pom.xml, it SHALL NOT
    // be included in the parsed dependency list.
    // ========================================================================
    
    /// Convert a dependency to a commented-out XML string
    fn dep_to_commented_xml(dep: &MavenDependency) -> String {
        format!("        <!-- {} -->\n", dep_to_xml(dep).trim())
    }
    
    /// Generate a pom.xml with some dependencies commented out
    fn deps_to_pom_with_comments(
        active_deps: &[MavenDependency], 
        commented_deps: &[MavenDependency]
    ) -> String {
        let mut pom = String::new();
        pom.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <dependencies>
"#);
        // Add active dependencies
        for dep in active_deps {
            pom.push_str(&dep_to_xml(dep));
        }
        // Add commented-out dependencies
        for dep in commented_deps {
            pom.push_str(&dep_to_commented_xml(dep));
        }
        pom.push_str("    </dependencies>\n</project>");
        pom
    }
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_xml_comment_handling(
            active_deps in arb_dependencies(),
            commented_deps in arb_dependencies()
        ) {
            // Generate pom.xml with both active and commented dependencies
            let pom_xml = deps_to_pom_with_comments(&active_deps, &commented_deps);
            
            // Parse the pom.xml
            let parsed_deps = parse_maven_pom(&pom_xml).expect("Should parse valid pom.xml");
            
            // Property: Only active (non-commented) dependencies should be parsed
            prop_assert_eq!(
                parsed_deps.len(), 
                active_deps.len(),
                "Should only parse active dependencies, not commented ones. \
                 Expected {} active deps, got {} parsed deps. \
                 Commented deps count: {}",
                active_deps.len(),
                parsed_deps.len(),
                commented_deps.len()
            );
            
            // Verify each parsed dependency matches an active dependency
            for (i, parsed) in parsed_deps.iter().enumerate() {
                let expected = &active_deps[i];
                prop_assert_eq!(
                    &parsed.group_id, 
                    &expected.group_id,
                    "groupId mismatch at index {}", i
                );
                prop_assert_eq!(
                    &parsed.artifact_id, 
                    &expected.artifact_id,
                    "artifactId mismatch at index {}", i
                );
            }
            
            // Additional check: commented dependencies should NOT appear in parsed list
            for commented in &commented_deps {
                let found = parsed_deps.iter().any(|p| {
                    p.group_id == commented.group_id && p.artifact_id == commented.artifact_id
                });
                // Note: This could be a false positive if the same dep is in both lists,
                // so we only check if the commented dep is NOT in active_deps
                let in_active = active_deps.iter().any(|a| {
                    a.group_id == commented.group_id && a.artifact_id == commented.artifact_id
                });
                if !in_active && found {
                    prop_assert!(false, 
                        "Commented dependency {}:{} should not be in parsed list",
                        commented.group_id, commented.artifact_id
                    );
                }
            }
        }
    }
    
    // ========================================================================
    // Property 9: Gradle Configuration Distinction
    // **Feature: java-perf-semantic-analysis, Property 9: Gradle Configuration Distinction**
    // **Validates: Requirements 3.4**
    // 
    // For any build.gradle with testImplementation dependencies, those SHALL be
    // classified with Test scope and not affect main stack detection.
    // ========================================================================
    
    /// Generate a Gradle configuration
    fn arb_gradle_configuration() -> impl Strategy<Value = GradleConfiguration> {
        prop_oneof![
            Just(GradleConfiguration::Implementation),
            Just(GradleConfiguration::TestImplementation),
            Just(GradleConfiguration::CompileOnly),
            Just(GradleConfiguration::RuntimeOnly),
            Just(GradleConfiguration::TestCompileOnly),
            Just(GradleConfiguration::TestRuntimeOnly),
            Just(GradleConfiguration::Api),
            Just(GradleConfiguration::AnnotationProcessor),
        ]
    }
    
    /// Generate a GradleDependency
    fn arb_gradle_dependency() -> impl Strategy<Value = GradleDependency> {
        (arb_group_id(), arb_artifact_id(), prop::option::of(arb_version()), arb_gradle_configuration())
            .prop_map(|(group, name, version, configuration)| {
                GradleDependency {
                    group,
                    name,
                    version,
                    configuration,
                }
            })
    }
    
    /// Generate a list of Gradle dependencies
    fn arb_gradle_dependencies() -> impl Strategy<Value = Vec<GradleDependency>> {
        prop::collection::vec(arb_gradle_dependency(), 0..10)
    }
    
    /// Convert a GradleDependency to Gradle DSL string
    fn gradle_dep_to_string(dep: &GradleDependency) -> String {
        let config_str = match &dep.configuration {
            GradleConfiguration::Implementation => "implementation",
            GradleConfiguration::TestImplementation => "testImplementation",
            GradleConfiguration::CompileOnly => "compileOnly",
            GradleConfiguration::RuntimeOnly => "runtimeOnly",
            GradleConfiguration::TestCompileOnly => "testCompileOnly",
            GradleConfiguration::TestRuntimeOnly => "testRuntimeOnly",
            GradleConfiguration::Api => "api",
            GradleConfiguration::AnnotationProcessor => "annotationProcessor",
            GradleConfiguration::Other(s) => s.as_str(),
        };
        
        let dep_str = if let Some(ref v) = dep.version {
            format!("{}:{}:{}", dep.group, dep.name, v)
        } else {
            format!("{}:{}", dep.group, dep.name)
        };
        
        format!("    {} '{}'\n", config_str, dep_str)
    }
    
    /// Convert a list of Gradle dependencies to a build.gradle file
    fn deps_to_gradle(deps: &[GradleDependency]) -> String {
        let mut gradle = String::new();
        gradle.push_str("plugins {\n    id 'java'\n}\n\ndependencies {\n");
        for dep in deps {
            gradle.push_str(&gradle_dep_to_string(dep));
        }
        gradle.push_str("}\n");
        gradle
    }
    
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_gradle_configuration_distinction(deps in arb_gradle_dependencies()) {
            // Generate build.gradle from dependencies
            let gradle_content = deps_to_gradle(&deps);
            
            // Parse the build.gradle
            let parsed_deps = parse_gradle_build(&gradle_content).expect("Should parse valid build.gradle");
            
            // Verify all dependencies were parsed correctly
            prop_assert_eq!(parsed_deps.len(), deps.len(), 
                "Should parse same number of dependencies. Expected {}, got {}.\nGradle content:\n{}",
                deps.len(), parsed_deps.len(), gradle_content);
            
            // Verify each parsed dependency has the correct configuration
            for (i, (parsed, expected)) in parsed_deps.iter().zip(deps.iter()).enumerate() {
                prop_assert_eq!(
                    &parsed.group, 
                    &expected.group,
                    "group mismatch at index {}", i
                );
                prop_assert_eq!(
                    &parsed.name, 
                    &expected.name,
                    "name mismatch at index {}", i
                );
                prop_assert_eq!(
                    &parsed.configuration, 
                    &expected.configuration,
                    "configuration mismatch at index {} for {}:{}", 
                    i, expected.group, expected.name
                );
            }
            
            // Detect stack from parsed dependencies
            let stack = detect_stack_from_gradle_deps(&parsed_deps);
            
            // Get only main (non-test) dependencies
            let main_deps: Vec<_> = deps.iter()
                .filter(|d| d.configuration.is_main_configuration())
                .collect();
            
            // Check that test-configuration reactive dependencies don't affect is_reactive
            let has_main_reactive = main_deps.iter().any(|d| {
                d.name == "spring-boot-starter-webflux" 
                    || d.name == "reactor-core"
                    || (d.group == "io.projectreactor" && d.name.starts_with("reactor-"))
            });
            
            // If no main-configuration reactive deps exist, is_reactive should be false
            if !has_main_reactive {
                prop_assert!(!stack.is_reactive, 
                    "is_reactive should be false when reactive deps are only in test configuration");
            }
            
            // Check that test-configuration spring-boot dependencies don't affect is_spring_boot
            let has_main_spring_boot = main_deps.iter().any(|d| {
                d.name.contains("spring-boot-starter") || d.group == "org.springframework.boot"
            });
            
            if !has_main_spring_boot {
                prop_assert!(!stack.is_spring_boot,
                    "is_spring_boot should be false when spring-boot deps are only in test configuration");
            }
        }
    }
}
