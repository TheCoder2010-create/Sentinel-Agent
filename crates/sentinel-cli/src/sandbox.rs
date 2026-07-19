use colored::*;
use sentinel_sandbox::SandboxPolicy;

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");

    match sub {
        "check" => cmd_check(&args[1..]),
        "config" => cmd_config(&args[1..]),
        "help" | "--help" | "-h" => {
            println!("{}", "Sandbox commands:".yellow().bold());
            println!("  sentinel sandbox check [path]     Check if path is allowed by sandbox");
            println!("  sentinel sandbox config           Show current sandbox configuration");
            Ok(())
        }
        _ => {
            eprintln!("{} Unknown sandbox subcommand: '{}'", "Error:".red().bold(), sub);
            std::process::exit(1);
        }
    }
}

fn cmd_check(args: &[String]) -> anyhow::Result<()> {
    let policy = SandboxPolicy::default();
    let path = args.first().map(|s| s.as_str()).unwrap_or(".");

    println!("{}", "Sandbox Check:".yellow().bold());
    println!("  Path:       {}", path);
    println!("  Can read:   {}", if policy.can_read(path) { "yes".green() } else { "no".red() });
    println!("  Can write:  {}", if policy.can_write(path) { "yes".green() } else { "no".red() });
    println!("  Can execute: {}", if policy.can_execute(path) { "yes".green() } else { "no".red() });
    println!("  Network:    {}", if policy.network { "allowed".green() } else { "blocked".red() });
    Ok(())
}

fn cmd_config(_args: &[String]) -> anyhow::Result<()> {
    println!("{}", "Sandbox Configuration:".yellow().bold());
    println!("  Sandboxing is policy-based at the application level.");
    println!("  Configure via sentinel.toml to restrict read/write/exec paths.");
    println!();
    println!("  Default policy allows all access (open mode).");
    println!("  Add [[sandbox.allowed_paths]] to restrict.");
    Ok(())
}
