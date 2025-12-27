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
use super::{Issue, Severity};
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
            })
        } else {
            None
        }
    }
}

/// N+1 检测处理器 - 带语义分析
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

        let is_suspicious = if let Some(symbol_table) = ctx.symbol_table {
            // Semantic Mode
            if !receiver_name.is_empty() {
                symbol_table.is_dao_call(ctx.current_class, &receiver_name, &method_name_text)
            } else {
                // Fallback
                method_name_text.contains("find") || method_name_text.contains("save")
            }
        } else {
            // Heuristic Mode
            Self::is_dao_method(&method_name_text) || Self::is_dao_receiver(&receiver_name)
        };

        if is_suspicious {
            // v9.4: 使用 CallGraph 验证调用链
            let call_chain_info = if let Some(cg) = ctx.call_graph {
                // 构建当前调用的方法签名
                let caller = crate::taint::MethodSig::new(ctx.current_class, "current_method");
                let paths = cg.trace_to_layer(&caller, crate::taint::LayerType::Repository, 5);
                
                if !paths.is_empty() {
                    // 找到了到 Repository 的调用链
                    let path_str: Vec<String> = paths[0].iter()
                        .map(|m| format!("{}.{}", m.class, m.name))
                        .collect();
                    Some(format!(" [调用链验证: {}]", path_str.join(" → ")))
                } else {
                    None
                }
            } else {
                None
            };

            let context_str = format!(
                "{}.{}(){}",
                receiver_name,
                method_name_text,
                call_chain_info.unwrap_or_default()
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
                });
            }
        }
        None
    }
}

/// ThreadLocal 泄漏检测处理器
pub struct ThreadLocalLeakHandler;

impl RuleHandler for ThreadLocalLeakHandler {
    fn handle(
        &self,
        query: &Query,
        m: &QueryMatch,
        rule_id: &str,
        severity: Severity,
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

        if let Some(method) = method_node {
            let method_text = method.utf8_text(ctx.code.as_bytes()).unwrap_or("");
            let remove_call = format!("{var_name}.remove()");

            if !method_text.contains(&remove_call) {
                let line = node.start_position().row + 1;
                return Some(Issue {
                    id: rule_id.to_string(),
                    severity,
                    file: ctx.file_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    line,
                    description: format!("{} (Variable: {})", description, var_name),
                    context: Some(var_name),
                });
            }
        }
        None
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
                        });
                    }
                }
            }
        }
        None
    }
}
