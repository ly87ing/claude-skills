use super::{CodeAnalyzer, Issue, Severity};
use std::path::Path;
use anyhow::{Result, anyhow};
use tree_sitter::{Parser, Query, QueryCursor};

/// 预编译的规则
struct CompiledRule {
    id: &'static str,
    severity: Severity,
    query: Query,
    description: &'static str,
}

pub struct JavaTreeSitterAnalyzer {
    language: tree_sitter::Language,
    /// 预编译的查询 (在 new() 时编译一次)
    compiled_rules: Vec<CompiledRule>,
}

impl JavaTreeSitterAnalyzer {
    pub fn new() -> Result<Self> {
        let language = tree_sitter_java::language();
        
        // 预编译所有查询
        let compiled_rules = Self::compile_rules(&language)?;
        
        Ok(Self {
            language,
            compiled_rules,
        })
    }

    /// 编译规则查询 (只在初始化时调用一次)
    fn compile_rules(language: &tree_sitter::Language) -> Result<Vec<CompiledRule>> {
        let rule_defs = vec![
            // 规则1: N_PLUS_ONE - for 循环内的调用
            ("N_PLUS_ONE", Severity::P0, r#"
                (for_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "for 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则1b: N_PLUS_ONE_WHILE - while 循环内的调用
            ("N_PLUS_ONE_WHILE", Severity::P0, r#"
                (while_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "while 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则1c: N_PLUS_ONE_FOREACH - 增强型 for 循环内的调用
            ("N_PLUS_ONE_FOREACH", Severity::P0, r#"
                (enhanced_for_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                name: (identifier) @method_name
                            ) @call
                        )
                    )
                )
            "#, "foreach 循环内调用方法 (可能是 N+1 问题)"),
            
            // 规则2: NESTED_LOOP - for 嵌套 for
            ("NESTED_LOOP", Severity::P0, r#"
                (for_statement
                    body: (block
                        (for_statement) @inner_loop
                    )
                )
            "#, "嵌套 for 循环 (可能导致 O(N^2) 复杂度)"),
            
            // 规则2b: NESTED_LOOP_FOREACH - for 嵌套 enhanced_for 或反之
            ("NESTED_LOOP_MIXED", Severity::P0, r#"
                [
                    (for_statement body: (block (enhanced_for_statement) @inner_loop))
                    (enhanced_for_statement body: (block (for_statement) @inner_loop))
                    (enhanced_for_statement body: (block (enhanced_for_statement) @inner_loop))
                ]
            "#, "嵌套循环 (可能导致 O(N^2) 复杂度)"),
            
            // 规则3: SYNC_METHOD (方法级同步)
            ("SYNC_METHOD", Severity::P0, r#"
                (method_declaration
                    (modifiers) @mods
                )
            "#, "Synchronized 方法级锁 (建议改用细粒度锁)"),
            
            // 规则4: THREADLOCAL_LEAK (P0)
            ("THREADLOCAL_LEAK", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @var_name
                    name: (identifier) @method
                    (#eq? @method "set")
                ) @set_call
            "#, "ThreadLocal.set() 后未在同一方法内调用 remove()"),
            
            // 规则5: STREAM_RESOURCE_LEAK - try 块内创建流但未在 finally 中关闭
            ("STREAM_RESOURCE_LEAK", Severity::P1, r#"
                (try_statement
                    body: (block
                        (local_variable_declaration
                            type: (_) @type_name
                            declarator: (variable_declarator
                                name: (identifier) @var_name
                                value: (object_creation_expression) @creation
                            )
                        )
                    )
                ) @try_block
            "#, "try 块内创建资源，请确保在 finally 中关闭或使用 try-with-resources"),
            
            // 规则6: SLEEP_IN_LOCK - synchronized 块内调用 sleep (P0)
            ("SLEEP_IN_LOCK", Severity::P0, r#"
                (synchronized_statement
                    body: (block
                        (expression_statement
                            (method_invocation
                                object: (identifier) @class_name
                                name: (identifier) @method_name
                                (#eq? @class_name "Thread")
                                (#eq? @method_name "sleep")
                            )
                        )
                    )
                ) @sync_block
            "#, "synchronized 块内调用 Thread.sleep()，持锁睡眠导致其他线程阻塞"),
            
            // 规则7: LOCK_METHOD_CALL - 检测 ReentrantLock.lock() 调用 (P0)
            ("LOCK_METHOD_CALL", Severity::P0, r#"
                (method_invocation
                    object: (identifier) @lock_var
                    name: (identifier) @method
                    (#eq? @method "lock")
                ) @lock_call
            "#, "ReentrantLock.lock() 调用，请确保 unlock() 在 finally 块中"),
        ];

        let mut compiled = Vec::with_capacity(rule_defs.len());
        
        for (id, severity, query_str, description) in rule_defs {
            let query = Query::new(language, query_str)
                .map_err(|e| anyhow!("Failed to compile query for {id}: {e}"))?;
            
            compiled.push(CompiledRule {
                id,
                severity,
                query,
                description,
            });
        }
        
        Ok(compiled)
    }
}

impl CodeAnalyzer for JavaTreeSitterAnalyzer {
    fn supported_extension(&self) -> &str {
        "java"
    }

    fn analyze(&self, code: &str, file_path: &Path) -> Result<Vec<Issue>> {
        let mut parser = Parser::new();
        parser.set_language(&self.language).map_err(|e| anyhow!("Failed to set language: {e}"))?;

        let tree = parser.parse(code, None).ok_or_else(|| anyhow!("Failed to parse code"))?;
        let root_node = tree.root_node();
        let mut issues = Vec::new();

        // 使用预编译的查询 (不再每次编译)
        for rule in &self.compiled_rules {
            let mut query_cursor = QueryCursor::new();
            let matches = query_cursor.matches(&rule.query, root_node, code.as_bytes());

            for m in matches {
                match rule.id {
                    // N+1 检测：支持 for, while, foreach 三种循环
                    "N_PLUS_ONE" | "N_PLUS_ONE_WHILE" | "N_PLUS_ONE_FOREACH" => {
                        let method_name_idx = rule.query.capture_index_for_name("method_name").unwrap();
                        let call_idx = rule.query.capture_index_for_name("call").unwrap();
                        let mut method_name_text = String::new();
                        let mut line = 0;
                        
                        for capture in m.captures {
                            if capture.index == method_name_idx {
                                method_name_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                            }
                            if capture.index == call_idx {
                                line = capture.node.start_position().row + 1;
                            }
                        }

                        // 检查是否是 DAO/RPC 方法名
                        if method_name_text.contains("find") || 
                           method_name_text.contains("save") || 
                           method_name_text.contains("select") || 
                           method_name_text.contains("delete") ||
                           method_name_text.contains("get") ||
                           method_name_text.contains("query") ||
                           method_name_text.contains("load") ||
                           method_name_text.contains("fetch") {
                            
                            let file_name = file_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            // 统一 ID 为 N_PLUS_ONE，便于上层处理
                            issues.push(Issue {
                                id: "N_PLUS_ONE".to_string(),
                                severity: rule.severity,
                                file: file_name,
                                line,
                                description: format!("{} (Method: {})", rule.description, method_name_text),
                                context: Some(method_name_text),
                            });
                        }
                    },
                    // 嵌套循环检测：支持 for-for, for-foreach, foreach-for, foreach-foreach
                    "NESTED_LOOP" | "NESTED_LOOP_MIXED" => {
                        let inner_loop_idx = rule.query.capture_index_for_name("inner_loop").unwrap();
                        for capture in m.captures {
                            if capture.index == inner_loop_idx {
                                let line = capture.node.start_position().row + 1;
                                // 统一 ID 为 NESTED_LOOP
                                issues.push(Issue {
                                    id: "NESTED_LOOP".to_string(),
                                    severity: rule.severity,
                                    file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                    line,
                                    description: rule.description.to_string(),
                                    context: None,
                                });
                            }
                        }
                    },
                    "SYNC_METHOD" => {
                        let mods_idx = rule.query.capture_index_for_name("mods").unwrap();
                        for capture in m.captures {
                            if capture.index == mods_idx {
                                let mods_text = capture.node.utf8_text(code.as_bytes()).unwrap_or("");
                                if mods_text.contains("synchronized") {
                                    let line = capture.node.start_position().row + 1;
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: rule.description.to_string(),
                                        context: Some(mods_text.to_string()),
                                    });
                                }
                            }
                        }
                    },
                    "THREADLOCAL_LEAK" => {
                        let set_call_idx = rule.query.capture_index_for_name("set_call").unwrap();
                        let var_name_idx = rule.query.capture_index_for_name("var_name").unwrap();
                        
                        let mut var_name = String::new();
                        let mut set_node = None;

                        for capture in m.captures {
                            if capture.index == var_name_idx {
                                var_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                            }
                            if capture.index == set_call_idx {
                                set_node = Some(capture.node);
                            }
                        }

                        if !var_name.is_empty() && set_node.is_some() {
                            let node = set_node.unwrap();
                            // 向上查找 method_declaration
                            let mut current = node.parent();
                            let mut method_node = None;
                            
                            while let Some(n) = current {
                                if n.kind() == "method_declaration" {
                                    method_node = Some(n);
                                    break;
                                }
                                current = n.parent();
                            }

                            if let Some(method) = method_node {
                                let method_text = method.utf8_text(code.as_bytes()).unwrap_or("");
                                let remove_call = format!("{var_name}.remove()");
                                
                                if !method_text.contains(&remove_call) {
                                     let line = node.start_position().row + 1;
                                     issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: format!("{} (Variable: {})", rule.description, var_name),
                                        context: Some(var_name),
                                    });
                                }
                            }
                        }
                    },
                    "STREAM_RESOURCE_LEAK" => {
                        // 检测 try 块内创建的流资源
                        if let Some(type_idx) = rule.query.capture_index_for_name("type_name") {
                            if let Some(var_idx) = rule.query.capture_index_for_name("var_name") {
                                let mut type_name = String::new();
                                let mut var_name = String::new();
                                let mut line = 0;

                                for capture in m.captures {
                                    if capture.index == type_idx {
                                        type_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                    }
                                    if capture.index == var_idx {
                                        var_name = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                        line = capture.node.start_position().row + 1;
                                    }
                                }

                                // 只关注流类型
                                if type_name.contains("Stream") || 
                                   type_name.contains("Reader") || 
                                   type_name.contains("Writer") ||
                                   type_name.contains("Connection") ||
                                   type_name.contains("Socket") {
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: format!("{} (Type: {}, Var: {})", rule.description, type_name, var_name),
                                        context: Some(var_name),
                                    });
                                }
                            }
                        }
                    },
                    "SLEEP_IN_LOCK" => {
                        // 检测 synchronized 块内的 Thread.sleep()
                        if let Some(sync_idx) = rule.query.capture_index_for_name("sync_block") {
                            for capture in m.captures {
                                if capture.index == sync_idx {
                                    let line = capture.node.start_position().row + 1;
                                    issues.push(Issue {
                                        id: rule.id.to_string(),
                                        severity: rule.severity,
                                        file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                        line,
                                        description: rule.description.to_string(),
                                        context: Some("Thread.sleep() in synchronized".to_string()),
                                    });
                                }
                            }
                        }
                    },
                    "LOCK_METHOD_CALL" => {
                        // 检测 ReentrantLock.lock() 调用
                        if let Some(lock_idx) = rule.query.capture_index_for_name("lock_call") {
                            if let Some(var_idx) = rule.query.capture_index_for_name("lock_var") {
                                let mut lock_var = String::new();
                                let mut line = 0;

                                for capture in m.captures {
                                    if capture.index == var_idx {
                                        lock_var = capture.node.utf8_text(code.as_bytes()).unwrap_or("").to_string();
                                    }
                                    if capture.index == lock_idx {
                                        line = capture.node.start_position().row + 1;
                                    }
                                }

                                // 检查方法内是否有配对的 unlock()
                                // 向上查找 method_declaration
                                if let Some(lock_node) = m.captures.iter().find(|c| c.index == lock_idx).map(|c| c.node) {
                                    let mut current = lock_node.parent();
                                    let mut method_node = None;
                                    
                                    while let Some(n) = current {
                                        if n.kind() == "method_declaration" {
                                            method_node = Some(n);
                                            break;
                                        }
                                        current = n.parent();
                                    }

                                    if let Some(method) = method_node {
                                        let method_text = method.utf8_text(code.as_bytes()).unwrap_or("");
                                        let unlock_in_finally = format!("{lock_var}.unlock()");
                                        let has_finally = method_text.contains("finally");
                                        
                                        // 如果没有 finally 块或 finally 中没有 unlock
                                        if !has_finally || !method_text.contains(&unlock_in_finally) {
                                            issues.push(Issue {
                                                id: rule.id.to_string(),
                                                severity: rule.severity,
                                                file: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                                line,
                                                description: format!("{} (Lock: {})", rule.description, lock_var),
                                                context: Some(lock_var),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_n_plus_one_detection() {
        let code = r#"
            public class Test {
                public void process() {
                    for (int i = 0; i < 10; i++) {
                        repository.save(i);
                        userDao.findById(i);
                        System.out.println(i);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, "N_PLUS_ONE");
        assert!(issues[0].context.as_ref().unwrap().contains("save"));
        
        assert_eq!(issues[1].id, "N_PLUS_ONE");
        assert!(issues[1].context.as_ref().unwrap().contains("findById"));
    }

    #[test]
    fn test_nested_loop_detection() {
        let code = r#"
            public class Test {
                public void process() {
                    for (int i = 0; i < 10; i++) {
                        for (int j = 0; j < 10; j++) {
                            // nested loop
                        }
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "NESTED_LOOP");
    }

    #[test]
    fn test_sync_method_detection() {
        let code = r#"
            public class Test {
                public synchronized void unsafeMethod() {
                    // heavy operation
                }
                
                public void safeMethod() {
                    synchronized(this) {
                        // block sync
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "SYNC_METHOD");
        assert!(issues[0].context.as_ref().unwrap().contains("synchronized"));
    }

    #[test]
    fn test_threadlocal_leak_detection() {
        // Case 1: Leak (set without remove)
        let leak_code = r#"
            public class LeakTest {
                private static final ThreadLocal<User> currentUser = new ThreadLocal<>();

                public void handleRequest() {
                    currentUser.set(new User());
                    // process...
                    // Missing remove()!
                }
            }
        "#;
        
        // Case 2: Safe (set with remove)
        let safe_code = r#"
            public class SafeTest {
                private static final ThreadLocal<User> context = new ThreadLocal<>();

                public void handleSafely() {
                    try {
                        context.set(new User());
                        // process...
                    } finally {
                        context.remove();
                    }
                }
            }
        "#;
        
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();

        let leak_issues = analyzer.analyze(leak_code, &PathBuf::from("LeakTest.java")).unwrap();
        assert_eq!(leak_issues.len(), 1, "Should detect leak");
        assert_eq!(leak_issues[0].id, "THREADLOCAL_LEAK");
        assert!(leak_issues[0].context.as_ref().unwrap().contains("currentUser"));

        let safe_issues = analyzer.analyze(safe_code, &PathBuf::from("SafeTest.java")).unwrap();
        assert_eq!(safe_issues.len(), 0, "Should NOT detect safe usage due to remove()");
    }

    #[test]
    fn test_n_plus_one_while_loop() {
        let code = r#"
            public class Test {
                public void process() {
                    Iterator<User> it = users.iterator();
                    while (it.hasNext()) {
                        User u = it.next();
                        orderDao.findByUserId(u.getId());
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "N_PLUS_ONE"), "Should detect N+1 in while loop");
    }

    #[test]
    fn test_n_plus_one_foreach_loop() {
        let code = r#"
            public class Test {
                public void process(List<User> users) {
                    for (User user : users) {
                        userRepository.save(user);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "N_PLUS_ONE"), "Should detect N+1 in foreach loop");
    }

    #[test]
    fn test_nested_loop_foreach_mixed() {
        let code = r#"
            public class Test {
                public void process(List<User> users, List<Order> orders) {
                    for (User user : users) {
                        for (Order order : orders) {
                            // O(N*M) 复杂度
                        }
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "NESTED_LOOP"), "Should detect nested foreach loops");
    }

    #[test]
    fn test_sleep_in_lock() {
        let code = r#"
            public class Test {
                private final Object lock = new Object();
                
                public void badMethod() {
                    synchronized(lock) {
                        Thread.sleep(1000);
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(code, &file).unwrap();

        assert!(issues.iter().any(|i| i.id == "SLEEP_IN_LOCK"), "Should detect Thread.sleep() in synchronized block");
    }

    #[test]
    fn test_reentrant_lock_leak() {
        // Case 1: Leak (lock without finally unlock)
        let leak_code = r#"
            public class Test {
                private ReentrantLock myLock = new ReentrantLock();
                
                public void badMethod() {
                    myLock.lock();
                    doSomething();
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(leak_code, &file).unwrap();

        // 打印调试信息
        for issue in &issues {
            println!("Found issue: {} - {}", issue.id, issue.description);
        }

        assert!(issues.iter().any(|i| i.id == "LOCK_METHOD_CALL"), "Should detect lock() without finally unlock()");
    }

    #[test]
    fn test_reentrant_lock_safe() {
        // Case 2: Safe (lock with finally unlock)
        let safe_code = r#"
            public class Test {
                private ReentrantLock lock = new ReentrantLock();
                
                public void safeMethod() {
                    lock.lock();
                    try {
                        doSomething();
                    } finally {
                        lock.unlock();
                    }
                }
            }
        "#;
        
        let file = PathBuf::from("Test.java");
        let analyzer = JavaTreeSitterAnalyzer::new().unwrap();
        let issues = analyzer.analyze(safe_code, &file).unwrap();

        assert!(!issues.iter().any(|i| i.id == "LOCK_METHOD_CALL"), "Should NOT detect when unlock() is in finally");
    }
}
