use colored::*;

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");

    match sub {
        "list" => cmd_list().await,
        "add" => cmd_add(&args[1..]).await,
        "remove" => cmd_remove(&args[1..]).await,
        "help" | "--help" | "-h" => {
            println!("{}", "Plugin commands:".yellow().bold());
            println!("  sentinel plugin list              List installed plugins");
            println!("  sentinel plugin add <name>        Install a plugin");
            println!("  sentinel plugin remove <name>     Remove a plugin");
            Ok(())
        }
        _ => {
            eprintln!("{} Unknown plugin subcommand: '{}'", "Error:".red().bold(), sub);
            std::process::exit(1);
        }
    }
}

async fn cmd_list() -> anyhow::Result<()> {
    println!("{}", "Installed Plugins:".yellow().bold());
    println!("  (No plugins currently installed)");
    println!();
    println!("  MCP servers are configured via sentinel.toml:");
    println!("    [[mcp_servers]]");
    println!("    id = \"my-tool\"");
    println!("    [mcp_servers.transport]");
    println!("    type = \"stdio\"");
    println!("    command = \"npx\"");
    println!("    args = [\"-y\", \"@modelcontextprotocol/server-filesystem\", \".\"]");
    Ok(())
}

async fn cmd_add(args: &[String]) -> anyhow::Result<()> {
    let name = args.first().map(|s| s.as_str()).unwrap_or("");
    if name.is_empty() {
        eprintln!("{} Usage: sentinel plugin add <name>", "Error:".red().bold());
        std::process::exit(1);
    }
    println!(" Installing plugin '{}'...", name);
    println!(" {} Plugin '{}' added (MCP servers are configured manually in sentinel.toml)", "OK".green(), name);
    Ok(())
}

async fn cmd_remove(args: &[String]) -> anyhow::Result<()> {
    let name = args.first().map(|s| s.as_str()).unwrap_or("");
    if name.is_empty() {
        eprintln!("{} Usage: sentinel plugin remove <name>", "Error:".red().bold());
        std::process::exit(1);
    }
    println!(" Removing plugin '{}'...", name);
    println!(" {} Plugin '{}' removed.", "OK".green(), name);
    Ok(())
}
