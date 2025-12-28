mod ast_engine;
mod forensic;
mod jdk_engine;
mod checklist;
mod scanner;
mod cli;
mod taint;
mod symbol_table;
mod project_detector;
mod rules;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use anyhow::Result;

// Re-export Command from cli module
use cli::Command;

/// Java Performance Diagnostics Tool
///
/// CLI 工具，通过 Bash 调用，默认输出人类可读格式
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "info")]
    log_level: String,

    /// 输出 JSON 格式 (默认输出人类可读的 Markdown)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    cli::handle_command(args.command, args.json)
}
