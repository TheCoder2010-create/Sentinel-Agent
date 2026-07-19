use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
}

fn default_max_file_size() -> u64 { 10 * 1024 * 1024 } // 10 MB

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            read_paths: vec![".".into()],
            write_paths: vec![".".into()],
            network: true,
            allowed_commands: vec![],
            max_file_size: default_max_file_size(),
        }
    }
}

impl SandboxPolicy {
    pub fn can_read(&self, path: &str) -> bool {
        self.read_paths.iter().any(|p| path.starts_with(p))
    }

    pub fn can_write(&self, path: &str) -> bool {
        self.write_paths.iter().any(|p| path.starts_with(p))
    }

    pub fn can_execute(&self, command: &str) -> bool {
        if self.allowed_commands.is_empty() {
            return true;
        }
        self.allowed_commands.iter().any(|c| command.starts_with(c))
    }

    pub fn strict() -> Self {
        Self {
            read_paths: vec![".".into()],
            write_paths: vec![],
            network: false,
            allowed_commands: vec!["python".into(), "node".into(), "cargo".into()],
            max_file_size: 1024 * 1024,
        }
    }
}
