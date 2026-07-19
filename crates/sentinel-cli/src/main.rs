use std::sync::Arc;
use colored::*;
use tracing_subscriber::EnvFilter;

mod approval;
mod display;
mod handler;

use approval::CliApprovalGate;
use display::{print_banner, print_divider};
use handler::CliEventHandler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::WARN.into()))
        .init();

    let args: Vec<String> = std::env::args().collect();

    let config = Arc::new(sentinel_config::SentinelConfig::load()
        .unwrap_or_default());

    let model_id = if args.len() >= 2 && !args[1].starts_with('-') {
        args[1].clone()
    } else {
        config.agent.default_model.clone()
    };

    let prompt = if args.len() >= 2 {
        args[1..].join(" ")
    } else {
        let mut input = String::new();
        eprintln!("{}", "Enter prompt (Ctrl+D to submit):".yellow());
        for line in std::io::stdin().lines() {
            match line {
                Ok(l) => {
                    if l.trim().is_empty() { break; }
                    input.push_str(&l);
                    input.push('\n');
                }
                Err(_) => break,
            }
        }
        input.trim().to_string()
    };

    if prompt.is_empty() {
        eprintln!("{} sentinel [model] \"your prompt\"", "Usage:".yellow().bold());
        std::process::exit(1);
    }

    let provider_info = config.providers()
        .iter()
        .find(|p| p.models.iter().any(|m| m.id == model_id))
        .or_else(|| config.providers().first())
        .cloned();

    let provider_info = match provider_info {
        Some(p) => p,
        None => {
            eprintln!("{} No provider found for model '{}'", "Error:".red().bold(), model_id);
            std::process::exit(1);
        }
    };

    let provider = Arc::new(
        sentinel_provider::ProviderKind::from_info(provider_info)?
    );

    // Build tool registry with built-in tools
    let mut tool_registry = sentinel_tools::ToolRegistry::new();

    // Register MCP tools from config
    let mcp_servers = config.mcp_servers();
    if !mcp_servers.is_empty() {
        println!(" {} MCP servers configured", format!("{}", mcp_servers.len()).yellow());
        let mcp_clients: Vec<Arc<sentinel_mcp::McpClient>> = mcp_servers.iter().map(|def| {
            Arc::new(sentinel_mcp::McpClient::new(&def.id, def.transport.clone()))
        }).collect();

        let count = sentinel_mcp::register_all_mcp_tools(&mut tool_registry, mcp_clients).await;
        if count > 0 {
            println!("   {} MCP tools registered", format!("{}", count).green());
        }
    }

    let tools = Arc::new(tool_registry);
    let agent = sentinel_core::Agent::new(provider, tools, config.clone())
        .with_event_handler(Arc::new(CliEventHandler));

    let mut thread = sentinel_core::AgentThread::new(
        config.agent.max_turns,
        config.agent.max_iterations,
        config.agent.yolo_mode,
    );

    print_banner();
    println!(" Model:  {}", model_id.green().bold());
    println!(" Yolo:   {}", if config.agent.yolo_mode { "yes".green() } else { "no".yellow() });
    println!(" Stream: {}", "yes".cyan());
    print_divider();

    let approval = if config.agent.yolo_mode {
        Box::new(sentinel_core::AutoApprovalGate) as Box<dyn sentinel_core::ApprovalGate>
    } else {
        Box::new(CliApprovalGate) as Box<dyn sentinel_core::ApprovalGate>
    };

    // Use streaming agent loop
    let result = agent.run_streaming(&mut thread, &prompt, approval.as_ref()).await;

    match result {
        Ok(output) => match output {
            sentinel_core::AgentOutput::Success { .. } => {}
            sentinel_core::AgentOutput::Error { message } => {
                eprintln!("{} {}", "Error:".red().bold(), message);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }

    let stats = format!("(turns: {}, iterations: {})", thread.turn, thread.iterations);
    println!("\n{} {}", "Done.".green().bold(), stats.dimmed());
    Ok(())
}
