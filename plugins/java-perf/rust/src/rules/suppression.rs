//! 规则抑制机制
//!
//! 支持以下方式抑制规则:
//!
//! 1. 注解抑制 (Java)
//!    @SuppressWarnings("java-perf:RULE_ID")
//!    @SuppressWarnings({"java-perf:RULE_ID1", "java-perf:RULE_ID2"})
//!
//! 2. 注释抑制 (任何文件)
//!    // java-perf-ignore: RULE_ID
//!    // java-perf-ignore: RULE_ID1, RULE_ID2
//!    // java-perf-ignore-next-line: RULE_ID
//!
//! 3. 文件级抑制
//!    // java-perf-ignore-file: RULE_ID
//!    // java-perf-ignore-file (抑制所有规则)

use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;
use regex::Regex;

/// 抑制前缀
const SUPPRESS_PREFIX: &str = "java-perf";

/// 抑制指令正则
static SUPPRESS_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    // 匹配: java-perf-ignore: RULE_ID 或 java-perf-ignore-next-line: RULE_ID
    Regex::new(r"java-perf-ignore(?:-next-line)?(?:-file)?:\s*([A-Z_,\s]+)").unwrap()
});

static SUPPRESS_ANNOTATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    // 匹配: @SuppressWarnings("java-perf:RULE_ID") 或数组形式
    Regex::new(r#"@SuppressWarnings\s*\(\s*(?:\{[^}]*\}|"[^"]*")\s*\)"#).unwrap()
});

static RULE_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"java-perf:([A-Z_]+)").unwrap()
});

/// 抑制类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionType {
    /// 当前行抑制
    Line,
    /// 下一行抑制
    NextLine,
    /// 整个文件抑制
    File,
}

/// 抑制记录
#[derive(Debug, Clone)]
pub struct Suppression {
    /// 抑制类型
    pub suppression_type: SuppressionType,
    /// 被抑制的规则 ID (空表示抑制所有)
    pub rule_ids: HashSet<String>,
    /// 所在行号
    pub line: usize,
}

/// 文件抑制上下文
#[derive(Debug, Default)]
pub struct SuppressionContext {
    /// 行号 -> 该行被抑制的规则
    line_suppressions: HashMap<usize, HashSet<String>>,
    /// 文件级抑制的规则
    file_suppressions: HashSet<String>,
    /// 是否抑制文件中的所有规则
    suppress_all_file: bool,
}

impl SuppressionContext {
    /// 从代码中解析抑制指令
    pub fn parse(code: &str) -> Self {
        let mut ctx = Self::default();

        for (line_num, line) in code.lines().enumerate() {
            let line_number = line_num + 1; // 1-based

            // 检查注释抑制
            if let Some(suppression) = parse_comment_suppression(line, line_number) {
                match suppression.suppression_type {
                    SuppressionType::Line => {
                        ctx.line_suppressions
                            .entry(line_number)
                            .or_default()
                            .extend(suppression.rule_ids);
                    }
                    SuppressionType::NextLine => {
                        ctx.line_suppressions
                            .entry(line_number + 1)
                            .or_default()
                            .extend(suppression.rule_ids);
                    }
                    SuppressionType::File => {
                        if suppression.rule_ids.is_empty() {
                            ctx.suppress_all_file = true;
                        } else {
                            ctx.file_suppressions.extend(suppression.rule_ids);
                        }
                    }
                }
            }

            // 检查注解抑制
            if let Some(rule_ids) = parse_annotation_suppression(line) {
                // 注解抑制应用于接下来的元素（方法/字段/类）
                // 这里简化处理，假设注解在声明的前一行
                ctx.line_suppressions
                    .entry(line_number + 1)
                    .or_default()
                    .extend(rule_ids);
            }
        }

        ctx
    }

    /// 检查指定规则在指定行是否被抑制
    pub fn is_suppressed(&self, rule_id: &str, line: usize) -> bool {
        // 文件级全部抑制
        if self.suppress_all_file {
            return true;
        }

        // 文件级特定规则抑制
        if self.file_suppressions.contains(rule_id) {
            return true;
        }

        // 行级抑制
        if let Some(suppressed_rules) = self.line_suppressions.get(&line) {
            // 空集合表示抑制所有
            if suppressed_rules.is_empty() || suppressed_rules.contains(rule_id) {
                return true;
            }
        }

        false
    }

    /// 检查文件是否完全被抑制
    pub fn is_file_suppressed(&self) -> bool {
        self.suppress_all_file
    }
}

/// 解析注释抑制
fn parse_comment_suppression(line: &str, line_number: usize) -> Option<Suppression> {
    // 支持两种形式:
    // 1. 纯注释行: // java-perf-ignore: RULE_ID
    // 2. 行内注释: code(); // java-perf-ignore: RULE_ID

    // 检查行中是否包含 java-perf-ignore 指令
    if !line.contains("java-perf-ignore") {
        return None;
    }

    // 尝试从行中提取抑制指令
    if let Some(captures) = SUPPRESS_COMMENT_REGEX.captures(line) {
        let rule_ids_str = captures.get(1)?.as_str();
        let rule_ids: HashSet<String> = rule_ids_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let suppression_type = if line.contains("ignore-file") {
            SuppressionType::File
        } else if line.contains("ignore-next-line") {
            SuppressionType::NextLine
        } else {
            SuppressionType::Line
        };

        return Some(Suppression {
            suppression_type,
            rule_ids,
            line: line_number,
        });
    }

    None
}

/// 解析注解抑制
fn parse_annotation_suppression(line: &str) -> Option<HashSet<String>> {
    if !line.contains("@SuppressWarnings") {
        return None;
    }

    let mut rule_ids = HashSet::new();

    // 提取所有 java-perf:RULE_ID
    for captures in RULE_ID_REGEX.captures_iter(line) {
        if let Some(rule_id) = captures.get(1) {
            rule_ids.insert(rule_id.as_str().to_string());
        }
    }

    if rule_ids.is_empty() {
        None
    } else {
        Some(rule_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_suppression_line() {
        let code = r#"
            public void process() {
                // java-perf-ignore: N_PLUS_ONE
                for (User u : users) {
                    repo.save(u);
                }
            }
        "#;

        let ctx = SuppressionContext::parse(code);
        // Line 3 has the comment, suppression applies to that line
        assert!(ctx.is_suppressed("N_PLUS_ONE", 3));
        assert!(!ctx.is_suppressed("NESTED_LOOP", 3));
    }

    #[test]
    fn test_comment_suppression_next_line() {
        let code = r#"
            public void process() {
                // java-perf-ignore-next-line: N_PLUS_ONE
                for (User u : users) {
                    repo.save(u);
                }
            }
        "#;

        let ctx = SuppressionContext::parse(code);
        // Comment is on line 3, so line 4 should be suppressed
        assert!(ctx.is_suppressed("N_PLUS_ONE", 4));
        assert!(!ctx.is_suppressed("N_PLUS_ONE", 3));
    }

    #[test]
    fn test_file_suppression() {
        let code = r#"
            // java-perf-ignore-file: N_PLUS_ONE, NESTED_LOOP
            public class Test {
                // ...
            }
        "#;

        let ctx = SuppressionContext::parse(code);
        assert!(ctx.is_suppressed("N_PLUS_ONE", 10));
        assert!(ctx.is_suppressed("NESTED_LOOP", 100));
        assert!(!ctx.is_suppressed("SYNC_METHOD", 10));
    }

    #[test]
    fn test_annotation_suppression() {
        let code = r#"
            @SuppressWarnings("java-perf:N_PLUS_ONE")
            public void process() {
                for (User u : users) {
                    repo.save(u);
                }
            }
        "#;

        let ctx = SuppressionContext::parse(code);
        // Annotation on line 2, applies to line 3
        assert!(ctx.is_suppressed("N_PLUS_ONE", 3));
    }

    #[test]
    fn test_multiple_rules_suppression() {
        let code = r#"
            // java-perf-ignore: N_PLUS_ONE, NESTED_LOOP
            public void process() {
            }
        "#;

        let ctx = SuppressionContext::parse(code);
        assert!(ctx.is_suppressed("N_PLUS_ONE", 2));
        assert!(ctx.is_suppressed("NESTED_LOOP", 2));
        assert!(!ctx.is_suppressed("SYNC_METHOD", 2));
    }
}
