# Tree-sitter Query Files

This directory contains Tree-sitter query files (`.scm`) for Java performance analysis.

## 文件说明

| 文件 | 描述 |
|------|------|
| `n_plus_one.scm` | N+1 问题检测：循环内的数据库/远程调用 |
| `sql_issues.scm` | SQL 问题检测：SELECT *、LIKE 前导通配符 |
| `concurrency.scm` | 并发问题检测：synchronized、锁泄漏、ThreadLocal |

## 使用方式

这些文件作为参考文档，实际的查询字符串仍然硬编码在 `tree_sitter_java.rs` 中以保持"单二进制文件"的优势。

未来可以通过 `include_str!` 宏在编译时加载这些文件：

```rust
const N_PLUS_ONE_QUERY: &str = include_str!("../resources/queries/n_plus_one.scm");
```

## 语法参考

- [Tree-sitter Query 语法](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries)
- [tree-sitter-java 节点类型](https://github.com/tree-sitter/tree-sitter-java/blob/master/src/node-types.json)
