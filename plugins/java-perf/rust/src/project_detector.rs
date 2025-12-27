// ============================================================================
// 项目侦测模块 - 识别技术栈与版本
// ============================================================================

use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};

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
        // 简单关键词匹配 (比 xml 解析快且健壮)
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
        
        // 尝试提取 JDK 版本
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
            if content.contains("org.springframework.boot") || content.contains("spring-boot-starter") {
                stack.is_spring_boot = true;
            }
            if content.contains("webflux") || content.contains("reactor") {
                stack.is_reactive = true;
            }
            
            // 尝试提取 JDK 版本
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
}
