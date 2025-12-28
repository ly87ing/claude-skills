// ============================================================================
// Java Performance Diagnostics Tool - Library Interface
// ============================================================================
//
// This module exposes the internal modules for integration testing.
// The main binary (main.rs) uses these modules directly.

pub mod ast_engine;
pub mod forensic;
pub mod jdk_engine;
pub mod checklist;
pub mod scanner;
pub mod cli;
pub mod taint;
pub mod symbol_table;
pub mod project_detector;
pub mod rules;
