#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            run_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Welcome to Sentinel AI, {}!", name)
}

#[tauri::command]
async fn run_agent(prompt: String) -> Result<String, String> {
    // Future: wire up sentinel-core agent here
    // let config = sentinel_config::SentinelConfig::default();
    // let provider = ...;
    // let tools = sentinel_tools::ToolRegistry::new();
    // let agent = sentinel_core::Agent::new(provider, tools, config);
    // let mut thread = sentinel_core::AgentThread::new(50, 100, false);
    // let result = agent.run(&mut thread, &prompt).await.map_err(|e| e.to_string())?;
    // Ok(result.to_string())
    Ok(format!("Sentinel agent will process: {}", prompt))
}
