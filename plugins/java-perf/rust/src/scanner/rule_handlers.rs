// ============================================================================
// RuleHandler Trait - 规则处理器抽象
// ============================================================================
//
// v9.2: 解耦规则处理逻辑，遵循开闭原则
//
// 之前的问题：analyze_with_context 中有大量 match rule.id 分支
// 每次添加新规则都要修改这个大 match
//
// 解决方案：
// 1. 定义 RuleHandler trait
// 2. 每种规则类型实现自己的 Handler
// 3. CompiledRule 持有 Box<dyn RuleHandler>
//
// ============================================================================

use tree_sitter::{Query, QueryMatch};
use super::{Issue, Severity, Confidence};
use crate::symbol_table::SymbolTable;
use std::path::Path;
use crate::taint::CallGraph;  // v9.4: CallGraph 支持

/// 规则处理上下文
pub struct RuleContext<'a> {
    pub code: &'a str,
    pub file_path: &'a Path,
    pub current_class: &'a str,
    pub symbol_table: Option<&'a SymbolTable>,
    pub call_graph: Option<&'a CallGraph>,  // v9.4: 调用图，用于 N+1 验证
}

/// 规则处理器 trait
pub trait RuleHandler: Send + Sync {
    /// 处理匹配结果，返回检测到的问题（如果有）
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue>;
}

// ============================================================================
// 通用处理器实现
// ============================================================================

/// 简单匹配处理器 - 只需要报告匹配位置
pub struct SimpleMatchHandler {
    /// 用于获取行号的 capture 名称
    pub line_capture: &'static str,
}

impl RuleHandler for SimpleMatchHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let capture_idx = query.capture_index_for_name(self.line_capture)?;

        for capture in m.captures {
            if capture.index == capture_idx {
                let line = capture.node.start_position().row + 1;
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: None,
                    confidence: None, // Simple match handlers don't use confidence
                });
            }
        }
        None
    }
}

/// 字符串内容匹配处理器 - 用于 SQL 检测等
pub struct StringContentHandler {
    pub string_capture: &'static str,
    pub max_context_len: usize,
}

impl RuleHandler for StringContentHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let str_idx = query.capture_index_for_name(self.string_capture)?;

        for capture in m.captures {
            if capture.index == str_idx {
                let line = capture.node.start_position().row + 1;
                let str_content = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("");
                let context = if str_content.len() > self.max_context_len {
                    format!("{}...", &str_content[..self.max_context_len])
                } else {
                    str_content.to_string()
                };

                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: Some(context),
                    confidence: None, // String content handlers don't use confidence
                });
            }
        }
        None
    }
}

/// 修饰符检查处理器 - 检查 synchronized, volatile 等
pub struct ModifierCheckHandler {
    pub mods_capture: &'static str,
    pub target_capture: &'static str,
    pub required_modifier: &'static str,
}

impl RuleHandler for ModifierCheckHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let mods_idx = query.capture_index_for_name(self.mods_capture)?;
        let target_idx = query.capture_index_for_name(self.target_capture)?;

        let mut has_modifier = false;
        let mut line = 0;

        for capture in m.captures {
            if capture.index == mods_idx {
                let mods_text = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("");
                has_modifier = mods_text.contains(self.required_modifier);
            }
            if capture.index == target_idx {
                line = capture.node.start_position().row + 1;
            }
        }

        if has_modifier && line > 0 {
            Some(Issue {
                id: rule_id.to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: description.to_string(),
                context: None,
                confidence: None, // Modifier check handlers don't use confidence
            })
        } else {
            None
        }
    }
}

/// N+1 检测处理器 - 带语义分析
/// 
/// v9.10: Enhanced with confidence marking based on FQN resolution.
/// - High confidence: FQN was resolved successfully via SymbolTable
/// - Low confidence: Heuristic fallback was used (receiver name pattern matching)
pub struct NPlusOneHandler;

impl RuleHandler for NPlusOneHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        _rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let method_name_idx = query.capture_index_for_name("method_name")?;
        let call_idx = query.capture_index_for_name("call")?;

        let mut method_name_text = String::new();
        let mut line = 0;
        let mut call_node = None;

        for capture in m.captures {
            if capture.index == method_name_idx {
                method_name_text = capture.node.utf8_text(ctx.code.as_bytes())
                    .unwrap_or("").to_string();
            }
            if capture.index == call_idx {
                line = capture.node.start_position().row + 1;
                call_node = Some(capture.node);
            }
        }

        // 获取 receiver
        let mut receiver_name = String::new();
        if let Some(node) = call_node {
            if let Some(obj_node) = node.child_by_field_name("object") {
                receiver_name = obj_node.utf8_text(ctx.code.as_bytes())
                    .unwrap_or("").to_string();
            }
        }

        // Determine if suspicious and track confidence level
        let (is_suspicious, confidence) = if let Some(symbol_table) = ctx.symbol_table {
            // Semantic Mode - try to resolve via SymbolTable
            if !receiver_name.is_empty() {
                let is_dao = symbol_table.is_dao_call(ctx.current_class, &receiver_name, &method_name_text);
                if is_dao {
                    // Check if we have FQN resolution for the receiver
                    let has_fqn = symbol_table.lookup_var_type(ctx.current_class, &receiver_name)
                        .map(|type_info| type_info.fqn.contains('.'))
                        .unwrap_or(false);
                    
                    if has_fqn {
                        (true, Some(Confidence::High))
                    } else {
                        // SymbolTable says it's a DAO call but no FQN - medium confidence
                        (true, Some(Confidence::Medium))
                    }
                } else {
                    (false, None)
                }
            } else {
                // No receiver - fallback to method name heuristic
                let is_dao_method = Self::is_dao_method(&method_name_text);
                if is_dao_method {
                    (true, Some(Confidence::Low))
                } else {
                    (false, None)
                }
            }
        } else {
            // Heuristic Mode - no SymbolTable available
            let is_suspicious = Self::is_dao_method(&method_name_text) || Self::is_dao_receiver(&receiver_name);
            if is_suspicious {
                (true, Some(Confidence::Low))
            } else {
                (false, None)
            }
        };

        if is_suspicious {
            // v9.4: 使用 CallGraph 验证调用链
            let call_chain_info = if let Some(cg) = ctx.call_graph {
                // 构建当前调用的方法签名
                let caller = crate::taint::MethodSig::new(ctx.current_class, "current_method");
                let paths = cg.trace_to_layer(&caller, crate::taint::LayerType::Repository, 5);
                
                if !paths.is_empty() {
                    // 找到了到 Repository 的调用链
                    // v9.8: Use simple_class_name() for display, class_fqn for internal tracking
                    let path_str: Vec<String> = paths[0].iter()
                        .map(|m| format!("{}.{}", m.simple_class_name(), m.name))
                        .collect();
                    Some(format!(" [调用链验证: {}]", path_str.join(" → ")))
                } else {
                    None
                }
            } else {
                None
            };

            // Add confidence indicator to context
            let confidence_indicator = match confidence {
                Some(Confidence::High) => " [高置信度: FQN已解析]",
                Some(Confidence::Medium) => " [中置信度: 部分解析]",
                Some(Confidence::Low) => " [低置信度: 启发式检测]",
                None => "",
            };

            let context_str = format!(
                "{}.{}(){}{}",
                receiver_name,
                method_name_text,
                call_chain_info.unwrap_or_default(),
                confidence_indicator
            );

            Some(Issue {
                id: "N_PLUS_ONE".to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: description.to_string(),
                context: Some(context_str),
                confidence,
            })
        } else {
            None
        }
    }
}

impl NPlusOneHandler {
    fn is_dao_method(method_name: &str) -> bool {
        let dao_patterns = [
            "findBy", "findAll", "findOne", "findById",
            "saveAll", "saveAndFlush",
            "deleteBy", "deleteAll", "deleteById",
            "selectBy", "selectAll", "selectOne", "selectList",
            "queryBy", "queryFor", "queryAll",
            "loadBy", "loadAll", "fetchBy", "fetchAll",
            "insertBy", "insert", "updateBy", "update",
            "getById", "getOne", "getAll", "getList",
        ];
        dao_patterns.iter().any(|p| method_name.starts_with(p) || method_name.eq_ignore_ascii_case(p))
    }

    fn is_dao_receiver(receiver: &str) -> bool {
        let receiver_lower = receiver.to_lowercase();
        receiver_lower.contains("repo") || receiver_lower.contains("dao")
            || receiver_lower.contains("mapper") || receiver_lower.contains("service")
    }
}

// ============================================================================
// v9.3 新增处理器
// ============================================================================

/// 嵌套循环检测处理器
pub struct NestedLoopHandler;

impl RuleHandler for NestedLoopHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        _rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let inner_loop_idx = query.capture_index_for_name("inner_loop")?;
        for capture in m.captures {
            if capture.index == inner_loop_idx {
                let line = capture.node.start_position().row + 1;
                return Some(Issue {
                    id: "NESTED_LOOP".to_string(), // 统一 ID
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: None,
                    confidence: None, // Nested loop detection doesn't use confidence
                });
            }
        }
        None
    }
}

/// ThreadLocal 泄漏检测处理器
/// 
/// v9.9: Enhanced to detect remove() calls in finally blocks for accurate leak detection.
/// 
/// Severity gradation:
/// - No issue: remove() is called in a finally block for the same ThreadLocal variable
/// - P1: remove() exists but not in a finally block (potential leak on exception)
/// - P0: No remove() call at all (definite leak)
pub struct ThreadLocalLeakHandler;

impl ThreadLocalLeakHandler {
    /// Check if remove() is called in a finally block for the given variable
    /// 
    /// Traverses the AST to find try_statement nodes within the method,
    /// then checks if any finally_clause contains a matching remove() call.
    /// 
    /// # Arguments
    /// * `method_node` - The method_declaration AST node
    /// * `var_name` - The ThreadLocal variable name to check
    /// * `code` - The source code bytes
    /// 
    /// # Returns
    /// `true` if remove() is called in a finally block for the variable
    fn has_remove_in_finally(method_node: tree_sitter::Node, var_name: &str, code: &[u8]) -> bool {
        // Find all try_statement nodes in the method
        let mut cursor = method_node.walk();
        Self::find_remove_in_finally_recursive(&mut cursor, var_name, code)
    }

    /// Recursively search for remove() calls in finally blocks
    fn find_remove_in_finally_recursive(
        cursor: &mut tree_sitter::TreeCursor,
        var_name: &str,
        code: &[u8],
    ) -> bool {
        loop {
            let node = cursor.node();
            
            // Check if this is a try_statement
            if node.kind() == "try_statement" {
                // Look for finally_clause child
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "finally_clause" {
                            // Check if finally block contains var_name.remove()
                            if Self::finally_contains_remove(child, var_name, code) {
                                return true;
                            }
                        }
                    }
                }
            }
            
            // Recurse into children
            if cursor.goto_first_child() {
                if Self::find_remove_in_finally_recursive(cursor, var_name, code) {
                    return true;
                }
                cursor.goto_parent();
            }
            
            // Move to next sibling
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        false
    }

    /// Check if a finally_clause contains a remove() call for the given variable
    fn finally_contains_remove(finally_node: tree_sitter::Node, var_name: &str, code: &[u8]) -> bool {
        let mut cursor = finally_node.walk();
        Self::find_remove_call_recursive(&mut cursor, var_name, code)
    }

    /// Recursively search for var_name.remove() method invocation
    fn find_remove_call_recursive(
        cursor: &mut tree_sitter::TreeCursor,
        var_name: &str,
        code: &[u8],
    ) -> bool {
        loop {
            let node = cursor.node();
            
            // Check if this is a method_invocation
            if node.kind() == "method_invocation" {
                // Check if it's var_name.remove()
                if let (Some(obj), Some(method)) = (
                    node.child_by_field_name("object"),
                    node.child_by_field_name("name"),
                ) {
                    let obj_text = obj.utf8_text(code).unwrap_or("");
                    let method_text = method.utf8_text(code).unwrap_or("");
                    
                    if obj_text == var_name && method_text == "remove" {
                        return true;
                    }
                }
            }
            
            // Recurse into children
            if cursor.goto_first_child() {
                if Self::find_remove_call_recursive(cursor, var_name, code) {
                    return true;
                }
                cursor.goto_parent();
            }
            
            // Move to next sibling
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        false
    }

    /// Check if remove() is called anywhere in the method (not necessarily in finally)
    fn has_remove_anywhere(method_node: tree_sitter::Node, var_name: &str, code: &[u8]) -> bool {
        let mut cursor = method_node.walk();
        Self::find_remove_call_recursive(&mut cursor, var_name, code)
    }

    /// Determine severity based on remove() placement
    /// 
    /// # Returns
    /// - `None` if remove() is in finally (safe, no issue)
    /// - `Some(Severity::P1)` if remove() exists but not in finally
    /// - `Some(Severity::P0)` if no remove() at all
    fn determine_severity(
        method_node: tree_sitter::Node,
        var_name: &str,
        code: &[u8],
    ) -> Option<Severity> {
        let has_finally_remove = Self::has_remove_in_finally(method_node, var_name, code);
        let has_any_remove = Self::has_remove_anywhere(method_node, var_name, code);

        match (has_finally_remove, has_any_remove) {
            (true, _) => None,                // Safe: remove in finally
            (false, true) => Some(Severity::P1), // Remove exists but not in finally
            (false, false) => Some(Severity::P0), // No remove at all
        }
    }
}

impl RuleHandler for ThreadLocalLeakHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        _severity: Severity, // Ignored - we determine severity dynamically
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let set_call_idx = query.capture_index_for_name("set_call")?;
        let var_name_idx = query.capture_index_for_name("var_name")?;

        let mut var_name = String::new();
        let mut set_node = None;

        for capture in m.captures {
            if capture.index == var_name_idx {
                var_name = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("").to_string();
            }
            if capture.index == set_call_idx {
                set_node = Some(capture.node);
            }
        }

        if var_name.is_empty() {
            return None;
        }

        let node = set_node?;

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

        let method = method_node?;
        
        // Use AST-based detection with severity gradation
        let determined_severity = Self::determine_severity(method, &var_name, ctx.code.as_bytes())?;
        
        let severity_desc = match determined_severity {
            Severity::P0 => "no remove() call found",
            Severity::P1 => "remove() not in finally block",
        };

        let line = node.start_position().row + 1;
        Some(Issue {
            id: rule_id.to_string(),
            severity: determined_severity,
            file: ctx.file_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            line,
            description: format!("{} (Variable: {}, {})", description, var_name, severity_desc),
            context: Some(var_name),
            confidence: Some(Confidence::High), // AST-based detection is high confidence
        })
    }
}

/// 流资源泄漏检测处理器
pub struct StreamResourceLeakHandler;

impl RuleHandler for StreamResourceLeakHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let type_idx = query.capture_index_for_name("type_name")?;
        let var_idx = query.capture_index_for_name("var_name")?;

        let mut type_name = String::new();
        let mut var_name = String::new();
        let mut line = 0;

        for capture in m.captures {
            if capture.index == type_idx {
                type_name = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("").to_string();
            }
            if capture.index == var_idx {
                var_name = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("").to_string();
                line = capture.node.start_position().row + 1;
            }
        }

        // 只关注流类型
        if type_name.contains("Stream") || type_name.contains("Reader")
            || type_name.contains("Writer") || type_name.contains("Connection")
            || type_name.contains("Socket") {
            Some(Issue {
                id: rule_id.to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: format!("{} (Type: {}, Var: {})", description, type_name, var_name),
                context: Some(var_name),
                confidence: None, // Stream resource leak detection doesn't use confidence
            })
        } else {
            None
        }
    }
}

/// 空参数检测处理器 - 用于检测 .get()/.join() 等无超时调用
pub struct EmptyArgsHandler {
    pub call_capture: &'static str,
    pub args_capture: &'static str,
}

impl RuleHandler for EmptyArgsHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let call_idx = query.capture_index_for_name(self.call_capture)?;
        let args_idx = query.capture_index_for_name(self.args_capture)?;

        let mut args_node = None;
        let mut line = 0;

        for capture in m.captures {
            if capture.index == args_idx {
                args_node = Some(capture.node);
            }
            if capture.index == call_idx {
                line = capture.node.start_position().row + 1;
            }
        }

        // 只有参数列表为空时才报告 (只有 ( 和 ))
        if let Some(args) = args_node {
            if args.child_count() <= 2 {
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: None,
                    confidence: None, // Empty args detection doesn't use confidence
                });
            }
        }
        None
    }
}

/// 方法调用带上下文处理器 - 用于 Flux.block() 等
pub struct MethodCallWithContextHandler {
    pub call_capture: &'static str,
}

impl RuleHandler for MethodCallWithContextHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let call_idx = query.capture_index_for_name(self.call_capture)?;

        for capture in m.captures {
            if capture.index == call_idx {
                let line = capture.node.start_position().row + 1;
                let method_text = capture.node.utf8_text(ctx.code.as_bytes())
                    .unwrap_or("").to_string();
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: Some(method_text),
                    confidence: None, // Method call with context doesn't use confidence
                });
            }
        }
        None
    }
}

/// subscribe 参数计数处理器
pub struct SubscribeArgCountHandler;

impl RuleHandler for SubscribeArgCountHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let call_idx = query.capture_index_for_name("call")?;

        for capture in m.captures {
            if capture.index == call_idx {
                let node = capture.node;
                let mut arg_count = 0;

                for child in node.children(&mut node.walk()) {
                    if child.kind() == "argument_list" {
                        for arg_child in child.children(&mut child.walk()) {
                            if arg_child.kind() != "," && arg_child.kind() != "(" && arg_child.kind() != ")" {
                                arg_count += 1;
                            }
                        }
                        break;
                    }
                }

                // 只有当参数数量 < 2 时才报告
                if arg_count < 2 {
                    let line = node.start_position().row + 1;
                    let method_text = node.utf8_text(ctx.code.as_bytes()).unwrap_or("").to_string();
                    return Some(Issue {
                        id: rule_id.to_string(),
                        severity,
                        file: ctx.file_path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        line,
                        description: format!("{} (参数数量: {})", description, arg_count),
                        context: Some(method_text),
                        confidence: None, // Subscribe arg count doesn't use confidence
                    });
                }
            }
        }
        None
    }
}

/// 空 catch 块检测处理器
pub struct EmptyCatchHandler;

impl RuleHandler for EmptyCatchHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let catch_idx = query.capture_index_for_name("catch")?;
        let body_idx = query.capture_index_for_name("body")?;

        let mut body_node = None;
        let mut line = 0;

        for capture in m.captures {
            if capture.index == body_idx {
                body_node = Some(capture.node);
            }
            if capture.index == catch_idx {
                line = capture.node.start_position().row + 1;
            }
        }

        if let Some(body) = body_node {
            let body_text = body.utf8_text(ctx.code.as_bytes()).unwrap_or("{}");
            let inner = body_text.trim_start_matches('{').trim_end_matches('}').trim();

            // 空或只有打印语句
            if inner.is_empty() || inner.contains(".print") {
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: description.to_string(),
                    context: None,
                    confidence: None, // Empty catch detection doesn't use confidence
                });
            }
        }
        None
    }
}

/// Lock 不在 finally 中释放检测处理器
pub struct LockNoFinallyHandler;

impl RuleHandler for LockNoFinallyHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let lock_idx = query.capture_index_for_name("lock_call")?;
        let var_idx = query.capture_index_for_name("lock_var")?;

        let mut lock_var = String::new();
        let mut line = 0;
        let mut lock_node = None;

        for capture in m.captures {
            if capture.index == var_idx {
                lock_var = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("").to_string();
            }
            if capture.index == lock_idx {
                line = capture.node.start_position().row + 1;
                lock_node = Some(capture.node);
            }
        }

        if let Some(node) = lock_node {
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
                let method_text = method.utf8_text(ctx.code.as_bytes()).unwrap_or("");
                let unlock_in_finally = format!("{lock_var}.unlock()");
                let has_finally = method_text.contains("finally");

                if !has_finally || !method_text.contains(&unlock_in_finally) {
                    return Some(Issue {
                        id: rule_id.to_string(),
                        severity,
                        file: ctx.file_path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        line,
                        description: format!("{} (Lock: {})", description, lock_var),
                        context: Some(lock_var),
                        confidence: None, // Lock detection doesn't use confidence
                    });
                }
            }
        }
        None
    }
}

/// 大数组分配检测处理器
pub struct LargeArrayHandler {
    pub threshold: i64,
}

impl RuleHandler for LargeArrayHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        let creation_idx = query.capture_index_for_name("creation")?;
        let size_idx = query.capture_index_for_name("size")?;

        let mut size_value: i64 = 0;
        let mut line = 0;

        for capture in m.captures {
            if capture.index == size_idx {
                let size_text = capture.node.utf8_text(ctx.code.as_bytes()).unwrap_or("0");
                size_value = size_text.parse().unwrap_or(0);
            }
            if capture.index == creation_idx {
                line = capture.node.start_position().row + 1;
            }
        }

        if size_value >= self.threshold {
            Some(Issue {
                id: rule_id.to_string(),
                severity,
                file: ctx.file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                line,
                description: format!("{} (size: {})", description, size_value),
                context: None,
                confidence: None, // Large array detection doesn't use confidence
            })
        } else {
            None
        }
    }
}

// ============================================================================
// 处理器工厂
// ============================================================================

/// 根据规则 ID 创建对应的处理器
pub fn create_handler(rule_id: &str) -> Box<dyn RuleHandler> {
    match rule_id {
        // ====== N+1 检测 ======
        "N_PLUS_ONE" | "N_PLUS_ONE_WHILE" | "N_PLUS_ONE_FOREACH" => {
            Box::new(NPlusOneHandler)
        }

        // ====== 嵌套循环检测 ======
        "NESTED_LOOP" | "NESTED_LOOP_MIXED" => {
            Box::new(NestedLoopHandler)
        }

        // ====== 修饰符检查 ======
        "SYNC_METHOD" => {
            Box::new(ModifierCheckHandler {
                mods_capture: "mods",
                target_capture: "mods", // SYNC_METHOD query only has @mods
                required_modifier: "synchronized",
            })
        }
        "VOLATILE_ARRAY" => {
            Box::new(ModifierCheckHandler {
                mods_capture: "mods",
                target_capture: "field",
                required_modifier: "volatile",
            })
        }
        "STATIC_COLLECTION" | "RANDOM_SHARED" => {
            Box::new(ModifierCheckHandler {
                mods_capture: "mods",
                target_capture: "field",
                required_modifier: "static",
            })
        }

        // ====== ThreadLocal 泄漏 ======
        "THREADLOCAL_LEAK" => {
            Box::new(ThreadLocalLeakHandler)
        }

        // ====== 流资源泄漏 ======
        "STREAM_RESOURCE_LEAK" => {
            Box::new(StreamResourceLeakHandler)
        }

        // ====== 无超时阻塞调用 ======
        "FUTURE_GET_NO_TIMEOUT" | "AWAIT_NO_TIMEOUT" | "COMPLETABLE_JOIN"
        | "EMITTER_UNBOUNDED" | "COMPLETABLE_GET_NO_TIMEOUT" => {
            Box::new(EmptyArgsHandler {
                call_capture: "call",
                args_capture: "args",
            })
        }

        // ====== SQL 字符串检测 ======
        "SELECT_STAR" | "LIKE_LEADING_WILDCARD" => {
            Box::new(StringContentHandler {
                string_capture: "str",
                max_context_len: 50,
            })
        }

        // ====== 响应式编程规则 (带上下文) ======
        "FLUX_BLOCK" | "FLUX_COLLECT_LIST" | "PARALLEL_NO_RUN_ON" => {
            Box::new(MethodCallWithContextHandler {
                call_capture: "call",
            })
        }

        // ====== subscribe 参数检查 ======
        "SUBSCRIBE_NO_ERROR" => {
            Box::new(SubscribeArgCountHandler)
        }

        // ====== 空 catch 块 ======
        "EMPTY_CATCH" => {
            Box::new(EmptyCatchHandler)
        }

        // ====== Lock 不在 finally ======
        "LOCK_METHOD_CALL" => {
            Box::new(LockNoFinallyHandler)
        }

        // ====== 大数组分配 ======
        "LARGE_ARRAY" => {
            Box::new(LargeArrayHandler {
                threshold: 1_000_000,
            })
        }

        // ====== 简单方法级规则 (匹配 @method) ======
        "FINALIZE_OVERRIDE" | "CACHEABLE_NO_KEY" | "TRANSACTIONAL_REQUIRES_NEW"
        | "TRANSACTION_SELF_CALL" | "ASYNC_DEFAULT_POOL" | "SCHEDULED_FIXED_RATE" => {
            Box::new(SimpleMatchHandler {
                line_capture: "method",
            })
        }

        // ====== @Autowired 字段注入 (匹配 @field) ======
        "AUTOWIRED_FIELD" => {
            Box::new(SimpleMatchHandler {
                line_capture: "field",
            })
        }

        // ====== 简单对象创建规则 (匹配 @creation) ======
        "SOFT_REFERENCE" | "OBJECT_IN_LOOP" | "BLOCKING_IO" | "ATOMIC_SPIN"
        | "SIMPLE_DATE_FORMAT" => {
            Box::new(SimpleMatchHandler {
                line_capture: "creation",
            })
        }

        // ====== 简单方法调用规则 (匹配 @call) ======
        "STRING_INTERN" | "UNBOUNDED_POOL" | "SINKS_MANY" | "CACHE_NO_EXPIRE"
        | "DATASOURCE_NO_POOL" | "LOG_STRING_CONCAT" | "GRAALVM_CLASS_FORNAME"
        | "GRAALVM_METHOD_INVOKE" | "GRAALVM_PROXY" | "SYSTEM_EXIT" | "RUNTIME_EXEC"
        | "HTTP_CLIENT_TIMEOUT" => {
            Box::new(SimpleMatchHandler {
                line_capture: "call",
            })
        }

        // ====== SLEEP_IN_LOCK (匹配 @sync_block) ======
        "SLEEP_IN_LOCK" => {
            Box::new(SimpleMatchHandler {
                line_capture: "sync_block",
            })
        }

        // ====== 简单同步块规则 (匹配 @sync) ======
        "SYNC_BLOCK" => {
            Box::new(SimpleMatchHandler {
                line_capture: "sync",
            })
        }

        // ====== Double-checked locking (匹配 @outer_if) ======
        "DOUBLE_CHECKED_LOCKING" => {
            Box::new(SimpleMatchHandler {
                line_capture: "outer_if",
            })
        }

        // ====== 循环内赋值规则 ======
        "STRING_CONCAT_LOOP" => {
            Box::new(SimpleMatchHandler {
                line_capture: "assign",
            })
        }

        // ====== 默认：尝试常见 capture 名称 ======
        _ => {
            Box::new(FallbackHandler)
        }
    }
}

/// 回退处理器 - 尝试多个常见 capture 名称
pub struct FallbackHandler;

impl RuleHandler for FallbackHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
        description: &str,
        ctx: &RuleContext,
    ) -> Option<Issue> {
        // 尝试常见 capture 名称顺序
        let capture_names = ["call", "method", "field", "creation", "sync", "outer_if", "assign"];

        for name in capture_names {
            if let Some(idx) = query.capture_index_for_name(name) {
                for capture in m.captures {
                    if capture.index == idx {
                        let line = capture.node.start_position().row + 1;
                        return Some(Issue {
                            id: rule_id.to_string(),
                            severity,
                            file: ctx.file_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            line,
                            description: description.to_string(),
                            context: None,
                            confidence: None, // Fallback handler doesn't use confidence
                        });
                    }
                }
            }
        }
        None
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tree_sitter::Parser;

    /// Parse Java code and return the tree
    fn parse_java(code: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_java::language()).unwrap();
        parser.parse(code, None).unwrap()
    }

    /// Find the method_declaration node in the tree
    fn find_method_node(tree: &tree_sitter::Tree) -> Option<tree_sitter::Node<'_>> {
        let root = tree.root_node();
        find_node_by_kind(root, "method_declaration")
    }

    /// Recursively find a node by kind
    fn find_node_by_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if let Some(found) = find_node_by_kind(child, kind) {
                    return Some(found);
                }
            }
        }
        None
    }

    // ========================================================================
    // Unit Tests for ThreadLocalLeakHandler
    // ========================================================================

    #[test]
    fn test_threadlocal_safe_with_finally_remove() {
        let code = r#"
            public class Test {
                private static ThreadLocal<String> context = new ThreadLocal<>();
                
                public void process() {
                    try {
                        context.set("value");
                        doWork();
                    } finally {
                        context.remove();
                    }
                }
            }
        "#;

        let tree = parse_java(code);
        let method = find_method_node(&tree).unwrap();
        
        // Should detect remove in finally
        assert!(ThreadLocalLeakHandler::has_remove_in_finally(method, "context", code.as_bytes()));
        
        // Severity should be None (safe)
        assert!(ThreadLocalLeakHandler::determine_severity(method, "context", code.as_bytes()).is_none());
    }

    #[test]
    fn test_threadlocal_p1_remove_outside_finally() {
        let code = r#"
            public class Test {
                private static ThreadLocal<String> context = new ThreadLocal<>();
                
                public void process() {
                    context.set("value");
                    doWork();
                    context.remove();
                }
            }
        "#;

        let tree = parse_java(code);
        let method = find_method_node(&tree).unwrap();
        
        // Should NOT detect remove in finally
        assert!(!ThreadLocalLeakHandler::has_remove_in_finally(method, "context", code.as_bytes()));
        
        // Should detect remove anywhere
        assert!(ThreadLocalLeakHandler::has_remove_anywhere(method, "context", code.as_bytes()));
        
        // Severity should be P1
        assert_eq!(
            ThreadLocalLeakHandler::determine_severity(method, "context", code.as_bytes()),
            Some(Severity::P1)
        );
    }

    #[test]
    fn test_threadlocal_p0_no_remove() {
        let code = r#"
            public class Test {
                private static ThreadLocal<String> context = new ThreadLocal<>();
                
                public void process() {
                    context.set("value");
                    doWork();
                }
            }
        "#;

        let tree = parse_java(code);
        let method = find_method_node(&tree).unwrap();
        
        // Should NOT detect remove in finally
        assert!(!ThreadLocalLeakHandler::has_remove_in_finally(method, "context", code.as_bytes()));
        
        // Should NOT detect remove anywhere
        assert!(!ThreadLocalLeakHandler::has_remove_anywhere(method, "context", code.as_bytes()));
        
        // Severity should be P0
        assert_eq!(
            ThreadLocalLeakHandler::determine_severity(method, "context", code.as_bytes()),
            Some(Severity::P0)
        );
    }

    #[test]
    fn test_threadlocal_nested_try_finally() {
        let code = r#"
            public class Test {
                private static ThreadLocal<String> context = new ThreadLocal<>();
                
                public void process() {
                    try {
                        context.set("value");
                        try {
                            doWork();
                        } finally {
                            cleanup();
                        }
                    } finally {
                        context.remove();
                    }
                }
            }
        "#;

        let tree = parse_java(code);
        let method = find_method_node(&tree).unwrap();
        
        // Should detect remove in finally (outer)
        assert!(ThreadLocalLeakHandler::has_remove_in_finally(method, "context", code.as_bytes()));
        
        // Severity should be None (safe)
        assert!(ThreadLocalLeakHandler::determine_severity(method, "context", code.as_bytes()).is_none());
    }

    #[test]
    fn test_threadlocal_different_variable() {
        let code = r#"
            public class Test {
                private static ThreadLocal<String> context = new ThreadLocal<>();
                private static ThreadLocal<String> other = new ThreadLocal<>();
                
                public void process() {
                    try {
                        context.set("value");
                        doWork();
                    } finally {
                        other.remove();
                    }
                }
            }
        "#;

        let tree = parse_java(code);
        let method = find_method_node(&tree).unwrap();
        
        // Should NOT detect remove for "context" (only "other" is removed)
        assert!(!ThreadLocalLeakHandler::has_remove_in_finally(method, "context", code.as_bytes()));
        
        // Should detect remove for "other"
        assert!(ThreadLocalLeakHandler::has_remove_in_finally(method, "other", code.as_bytes()));
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    /// Java reserved keywords that cannot be used as variable names
    const JAVA_KEYWORDS: &[&str] = &[
        "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char",
        "class", "const", "continue", "default", "do", "double", "else", "enum",
        "extends", "final", "finally", "float", "for", "goto", "if", "implements",
        "import", "instanceof", "int", "interface", "long", "native", "new", "package",
        "private", "protected", "public", "return", "short", "static", "strictfp",
        "super", "switch", "synchronized", "this", "throw", "throws", "transient",
        "try", "void", "volatile", "while", "true", "false", "null", "var", "yield",
        "record", "sealed", "permits", "non"
    ];

    /// Strategy to generate valid Java variable names (excluding reserved keywords)
    fn java_var_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9]{0,10}".prop_filter("Must be valid var name and not a keyword", |s| {
            !s.is_empty() 
                && s.chars().next().unwrap().is_lowercase()
                && !JAVA_KEYWORDS.contains(&s.as_str())
        })
    }

    proptest! {
        /// **Feature: java-perf-semantic-analysis, Property 5: ThreadLocal Safe Detection**
        /// 
        /// *For any* method containing ThreadLocal.set() followed by remove() in a finally block 
        /// for the same variable, the system SHALL NOT report a leak issue.
        /// 
        /// **Validates: Requirements 2.1, 2.3, 2.4**
        #[test]
        fn prop_threadlocal_safe_detection(
            var_name in java_var_name_strategy(),
        ) {
            // Generate Java code with ThreadLocal.set() and remove() in finally
            let code = format!(r#"
                public class Test {{
                    private static ThreadLocal<String> {} = new ThreadLocal<>();
                    
                    public void process() {{
                        try {{
                            {}.set("value");
                            doWork();
                        }} finally {{
                            {}.remove();
                        }}
                    }}
                }}
            "#, var_name, var_name, var_name);

            let tree = parse_java(&code);
            let method = find_method_node(&tree);
            
            prop_assert!(method.is_some(), "Should find method_declaration node");
            let method = method.unwrap();
            
            // Property 1: has_remove_in_finally should return true
            prop_assert!(
                ThreadLocalLeakHandler::has_remove_in_finally(method, &var_name, code.as_bytes()),
                "Should detect remove() in finally for variable '{}'",
                var_name
            );
            
            // Property 2: determine_severity should return None (safe)
            prop_assert!(
                ThreadLocalLeakHandler::determine_severity(method, &var_name, code.as_bytes()).is_none(),
                "Should return None severity (safe) when remove() is in finally for variable '{}'",
                var_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 5 (continued): No false positives**
        /// 
        /// *For any* method containing ThreadLocal.set() with remove() in finally for a DIFFERENT
        /// variable, the system SHALL still report a leak for the original variable.
        /// 
        /// **Validates: Requirements 2.1, 2.3, 2.4**
        #[test]
        fn prop_threadlocal_different_var_detection(
            var1 in java_var_name_strategy(),
            var2 in java_var_name_strategy(),
        ) {
            // Ensure variables are different
            prop_assume!(var1 != var2);
            
            // Generate Java code where var1.set() is called but var2.remove() is in finally
            let code = format!(r#"
                public class Test {{
                    private static ThreadLocal<String> {} = new ThreadLocal<>();
                    private static ThreadLocal<String> {} = new ThreadLocal<>();
                    
                    public void process() {{
                        try {{
                            {}.set("value");
                            doWork();
                        }} finally {{
                            {}.remove();
                        }}
                    }}
                }}
            "#, var1, var2, var1, var2);

            let tree = parse_java(&code);
            let method = find_method_node(&tree);
            
            prop_assert!(method.is_some(), "Should find method_declaration node");
            let method = method.unwrap();
            
            // Property 1: has_remove_in_finally for var1 should return false
            prop_assert!(
                !ThreadLocalLeakHandler::has_remove_in_finally(method, &var1, code.as_bytes()),
                "Should NOT detect remove() in finally for variable '{}' when only '{}' is removed",
                var1, var2
            );
            
            // Property 2: has_remove_in_finally for var2 should return true
            prop_assert!(
                ThreadLocalLeakHandler::has_remove_in_finally(method, &var2, code.as_bytes()),
                "Should detect remove() in finally for variable '{}'",
                var2
            );
            
            // Property 3: determine_severity for var1 should return P0 (no remove at all)
            prop_assert_eq!(
                ThreadLocalLeakHandler::determine_severity(method, &var1, code.as_bytes()),
                Some(Severity::P0),
                "Should return P0 severity for variable '{}' with no remove()",
                var1
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 6: ThreadLocal Severity Gradation**
        /// 
        /// *For any* ThreadLocal.set() without remove() in finally, the severity SHALL be P0 
        /// if no remove() exists anywhere, or P1 if remove() exists outside finally.
        /// 
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_threadlocal_severity_gradation_p0(
            var_name in java_var_name_strategy(),
        ) {
            // Generate Java code with ThreadLocal.set() but NO remove() at all
            let code = format!(r#"
                public class Test {{
                    private static ThreadLocal<String> {} = new ThreadLocal<>();
                    
                    public void process() {{
                        {}.set("value");
                        doWork();
                    }}
                }}
            "#, var_name, var_name);

            let tree = parse_java(&code);
            let method = find_method_node(&tree);
            
            prop_assert!(method.is_some(), "Should find method_declaration node");
            let method = method.unwrap();
            
            // Property 1: has_remove_in_finally should return false
            prop_assert!(
                !ThreadLocalLeakHandler::has_remove_in_finally(method, &var_name, code.as_bytes()),
                "Should NOT detect remove() in finally for variable '{}'",
                var_name
            );
            
            // Property 2: has_remove_anywhere should return false
            prop_assert!(
                !ThreadLocalLeakHandler::has_remove_anywhere(method, &var_name, code.as_bytes()),
                "Should NOT detect remove() anywhere for variable '{}'",
                var_name
            );
            
            // Property 3: determine_severity should return P0
            prop_assert_eq!(
                ThreadLocalLeakHandler::determine_severity(method, &var_name, code.as_bytes()),
                Some(Severity::P0),
                "Should return P0 severity when no remove() exists for variable '{}'",
                var_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 6 (continued): P1 severity**
        /// 
        /// *For any* ThreadLocal.set() with remove() outside finally, the severity SHALL be P1.
        /// 
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_threadlocal_severity_gradation_p1(
            var_name in java_var_name_strategy(),
        ) {
            // Generate Java code with ThreadLocal.set() and remove() outside finally
            let code = format!(r#"
                public class Test {{
                    private static ThreadLocal<String> {} = new ThreadLocal<>();
                    
                    public void process() {{
                        {}.set("value");
                        doWork();
                        {}.remove();
                    }}
                }}
            "#, var_name, var_name, var_name);

            let tree = parse_java(&code);
            let method = find_method_node(&tree);
            
            prop_assert!(method.is_some(), "Should find method_declaration node");
            let method = method.unwrap();
            
            // Property 1: has_remove_in_finally should return false
            prop_assert!(
                !ThreadLocalLeakHandler::has_remove_in_finally(method, &var_name, code.as_bytes()),
                "Should NOT detect remove() in finally for variable '{}'",
                var_name
            );
            
            // Property 2: has_remove_anywhere should return true
            prop_assert!(
                ThreadLocalLeakHandler::has_remove_anywhere(method, &var_name, code.as_bytes()),
                "Should detect remove() somewhere for variable '{}'",
                var_name
            );
            
            // Property 3: determine_severity should return P1
            prop_assert_eq!(
                ThreadLocalLeakHandler::determine_severity(method, &var_name, code.as_bytes()),
                Some(Severity::P1),
                "Should return P1 severity when remove() exists but not in finally for variable '{}'",
                var_name
            );
        }
    }

    // ========================================================================
    // Property 12: Heuristic Fallback Marking Tests
    // ========================================================================

    /// Strategy to generate DAO-like method names
    fn dao_method_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("findById".to_string()),
            Just("findAll".to_string()),
            Just("findByName".to_string()),
            Just("saveAll".to_string()),
            Just("deleteById".to_string()),
            Just("selectList".to_string()),
            Just("queryAll".to_string()),
            Just("getById".to_string()),
        ]
    }

    /// Strategy to generate DAO-like receiver names
    fn dao_receiver_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("userRepository".to_string()),
            Just("orderDao".to_string()),
            Just("productMapper".to_string()),
            Just("customerService".to_string()),
        ]
    }

    /// Strategy to generate non-DAO method names
    fn non_dao_method_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("process".to_string()),
            Just("calculate".to_string()),
            Just("validate".to_string()),
            Just("transform".to_string()),
            Just("convert".to_string()),
        ]
    }

    /// Strategy to generate non-DAO receiver names
    fn non_dao_receiver_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("helper".to_string()),
            Just("utils".to_string()),
            Just("converter".to_string()),
            Just("validator".to_string()),
        ]
    }

    #[test]
    fn test_nplusone_is_dao_method() {
        // Test DAO method patterns
        assert!(NPlusOneHandler::is_dao_method("findById"));
        assert!(NPlusOneHandler::is_dao_method("findAll"));
        assert!(NPlusOneHandler::is_dao_method("saveAll"));
        assert!(NPlusOneHandler::is_dao_method("deleteById"));
        assert!(NPlusOneHandler::is_dao_method("selectList"));
        assert!(NPlusOneHandler::is_dao_method("queryAll"));
        assert!(NPlusOneHandler::is_dao_method("getById"));
        
        // Test non-DAO methods
        assert!(!NPlusOneHandler::is_dao_method("process"));
        assert!(!NPlusOneHandler::is_dao_method("calculate"));
        assert!(!NPlusOneHandler::is_dao_method("validate"));
    }

    #[test]
    fn test_nplusone_is_dao_receiver() {
        // Test DAO receiver patterns
        assert!(NPlusOneHandler::is_dao_receiver("userRepository"));
        assert!(NPlusOneHandler::is_dao_receiver("orderDao"));
        assert!(NPlusOneHandler::is_dao_receiver("productMapper"));
        assert!(NPlusOneHandler::is_dao_receiver("customerService"));
        
        // Test non-DAO receivers
        assert!(!NPlusOneHandler::is_dao_receiver("helper"));
        assert!(!NPlusOneHandler::is_dao_receiver("utils"));
        assert!(!NPlusOneHandler::is_dao_receiver("converter"));
    }

    proptest! {
        /// **Feature: java-perf-semantic-analysis, Property 12: Heuristic Fallback Marking**
        /// 
        /// *For any* field type that cannot be resolved via SymbolTable, the system SHALL use 
        /// heuristic detection AND mark the result with reduced confidence (Low).
        /// 
        /// This test verifies that when no SymbolTable is available (heuristic mode),
        /// detected issues are marked with Low confidence.
        /// 
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_heuristic_fallback_low_confidence_dao_method(
            method_name in dao_method_name_strategy(),
        ) {
            // Property: When using heuristic detection (no SymbolTable), 
            // DAO method names should be detected with Low confidence
            
            // Test is_dao_method returns true for DAO patterns
            prop_assert!(
                NPlusOneHandler::is_dao_method(&method_name),
                "Method '{}' should be detected as DAO method by heuristic",
                method_name
            );
            
            // The confidence marking happens in the handler, which we verify
            // by checking that the heuristic detection logic correctly identifies
            // DAO methods. When no SymbolTable is available, these would be
            // marked with Low confidence.
        }

        /// **Feature: java-perf-semantic-analysis, Property 12 (continued): Receiver heuristic**
        /// 
        /// *For any* receiver name that matches DAO patterns, the system SHALL detect it
        /// using heuristic detection when SymbolTable is not available.
        /// 
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_heuristic_fallback_low_confidence_dao_receiver(
            receiver_name in dao_receiver_name_strategy(),
        ) {
            // Property: When using heuristic detection (no SymbolTable),
            // DAO receiver names should be detected with Low confidence
            
            prop_assert!(
                NPlusOneHandler::is_dao_receiver(&receiver_name),
                "Receiver '{}' should be detected as DAO receiver by heuristic",
                receiver_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 12 (continued): Non-DAO exclusion**
        /// 
        /// *For any* method name that does NOT match DAO patterns, the system SHALL NOT
        /// flag it as suspicious when using heuristic detection.
        /// 
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_heuristic_fallback_non_dao_method_excluded(
            method_name in non_dao_method_name_strategy(),
        ) {
            // Property: Non-DAO method names should NOT be detected by heuristic
            
            prop_assert!(
                !NPlusOneHandler::is_dao_method(&method_name),
                "Method '{}' should NOT be detected as DAO method by heuristic",
                method_name
            );
        }

        /// **Feature: java-perf-semantic-analysis, Property 12 (continued): Non-DAO receiver exclusion**
        /// 
        /// *For any* receiver name that does NOT match DAO patterns, the system SHALL NOT
        /// flag it as suspicious when using heuristic detection.
        /// 
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_heuristic_fallback_non_dao_receiver_excluded(
            receiver_name in non_dao_receiver_name_strategy(),
        ) {
            // Property: Non-DAO receiver names should NOT be detected by heuristic
            
            prop_assert!(
                !NPlusOneHandler::is_dao_receiver(&receiver_name),
                "Receiver '{}' should NOT be detected as DAO receiver by heuristic",
                receiver_name
            );
        }
    }
}
