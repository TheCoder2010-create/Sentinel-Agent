use crate::provider::{AuthConfig, ModelEntry, ProviderInfo};
use std::collections::HashMap;

pub fn default_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "openai".into(),
            name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            auth: AuthConfig::EnvKey { var: "OPENAI_API_KEY".into() },
            models: vec![
                ModelEntry { id: "gpt-4o".into(), name: "GPT-4o".into(), context_window: 128000, supports_streaming: true, supports_tools: true },
                ModelEntry { id: "gpt-4o-mini".into(), name: "GPT-4o Mini".into(), context_window: 128000, supports_streaming: true, supports_tools: true },
                ModelEntry { id: "o3-mini".into(), name: "o3 Mini".into(), context_window: 200000, supports_streaming: true, supports_tools: true },
            ],
            timeout_secs: 120,
            extra_headers: HashMap::new(),
        },
        ProviderInfo {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            auth: AuthConfig::EnvKey { var: "ANTHROPIC_API_KEY".into() },
            models: vec![
                ModelEntry { id: "claude-sonnet-4-20250514".into(), name: "Claude Sonnet 4".into(), context_window: 200000, supports_streaming: true, supports_tools: true },
                ModelEntry { id: "claude-haiku-3-5-20241022".into(), name: "Claude Haiku 3.5".into(), context_window: 200000, supports_streaming: true, supports_tools: true },
            ],
            timeout_secs: 180,
            extra_headers: HashMap::new(),
        },
        ProviderInfo {
            id: "google-ai-studio".into(),
            name: "Google AI Studio".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            auth: AuthConfig::EnvKey { var: "GOOGLE_API_KEY".into() },
            models: vec![
                ModelEntry { id: "gemini-2.5-flash".into(), name: "Gemini 2.5 Flash".into(), context_window: 1000000, supports_streaming: true, supports_tools: true },
                ModelEntry { id: "gemini-2.5-pro".into(), name: "Gemini 2.5 Pro".into(), context_window: 1000000, supports_streaming: true, supports_tools: true },
            ],
            timeout_secs: 120,
            extra_headers: HashMap::new(),
        },
        ProviderInfo {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com".into(),
            auth: AuthConfig::EnvKey { var: "DEEPSEEK_API_KEY".into() },
            models: vec![
                ModelEntry { id: "deepseek-chat".into(), name: "DeepSeek V3".into(), context_window: 64000, supports_streaming: true, supports_tools: true },
                ModelEntry { id: "deepseek-reasoner".into(), name: "DeepSeek R1".into(), context_window: 64000, supports_streaming: true, supports_tools: false },
            ],
            timeout_secs: 120,
            extra_headers: HashMap::new(),
        },
    ]
}
