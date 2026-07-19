use std::sync::Arc;
use sentinel_protocol::{
    CompletionRequest, Message, ContentBlock, Role, ToolResult,
};
use sentinel_provider::{ModelProvider, ProviderError};
use sentinel_tools::{ToolRegistry, ToolContext};
use sentinel_config::SentinelConfig;
use crate::thread::{AgentThread, ThreadStatus};

const SYSTEM_PROMPT: &str = r#"You are Sentinel, a coding agent. You help users with software engineering tasks.

You have access to tools that let you read, write, and edit files, execute commands, search code, and search the web.

When you need to use a tool, respond with a tool call. When you have completed the task, provide a summary of what you did.

Guidelines:
- Read files before editing them to understand their content
- Run tests after making changes to verify correctness
- Ask for clarification when instructions are ambiguous
- Use the bash tool for running commands, building, testing
- Use web_search for finding information"#;

pub struct Agent {
    provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
    config: Arc<SentinelConfig>,
}

impl Agent {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
        config: Arc<SentinelConfig>,
    ) -> Self {
        Self { provider, tools, config }
    }

    pub async fn run(&self, thread: &mut AgentThread, user_input: &str) -> AgentResult {
        thread.status = ThreadStatus::Running;
        thread.add_message(Message::user(user_input));

        if !thread.context.messages().iter().any(|m| m.role == Role::System) {
            thread.add_message(Message::system(SYSTEM_PROMPT));
        }

        loop {
            if !thread.increment_iteration() {
                return Ok(AgentOutput::error("Max iterations reached"));
            }

            let req = self.build_request(thread);
            let tool_defs = self.tools.tool_defs_for_model(true);

            let req = if let Some(tools) = tool_defs {
                req.with_tools(tools)
            } else {
                req
            };

            let response = match self.provider.complete(&req).await {
                Ok(r) => r,
                Err(e) => return Ok(AgentOutput::error(format!("LLM call failed: {}", e))),
            };

            let choice = match response.choices.into_iter().next() {
                Some(c) => c,
                None => return Ok(AgentOutput::error("No response from model")),
            };

            thread.add_message(choice.message.clone());
            let last_text = choice.message.extract_text();

            let tool_calls: Vec<_> = choice.message.content.iter()
                .filter_map(|b| {
                    if let ContentBlock::ToolCall { id, name, arguments } = b {
                        Some((id.clone(), name.clone(), arguments.clone()))
                    } else { None }
                })
                .collect();

            if tool_calls.is_empty() {
                thread.status = ThreadStatus::Completed;
                return Ok(AgentOutput::success(last_text));
            }

            let ctx = ToolContext::new();
            let mut tool_results = Vec::new();

            for (tool_call_id, name, args) in &tool_calls {
                if !thread.yolo_mode {
                    thread.status = ThreadStatus::AwaitingApproval;
                }

                let output = self.tools.execute(name, args.clone(), &ctx).await;
                tool_results.push(ToolResult {
                    tool_call_id: tool_call_id.clone(),
                    name: name.clone(),
                    output: output.text,
                    is_error: output.is_error,
                });
            }

            for result in &tool_results {
                thread.add_message(Message::new(Role::Tool, vec![
                    ContentBlock::ToolResult {
                        tool_call_id: result.tool_call_id.clone(),
                        content: result.output.clone(),
                        is_error: Some(result.is_error),
                    }
                ]));
            }

            if !thread.increment_turn() {
                return Ok(AgentOutput::error("Max turns reached"));
            }

            if thread.is_doom_loop() {
                return Ok(AgentOutput::error("Doom loop detected"));
            }

            if thread.context.needs_compaction() {
                thread.context.compact();
            }
        }
    }

    pub async fn run_stream(
        &self,
        thread: &mut AgentThread,
        user_input: &str,
    ) -> Result<AgentOutputStream, ProviderError> {
        thread.status = ThreadStatus::Running;
        thread.add_message(Message::user(user_input));

        if !thread.context.messages().iter().any(|m| m.role == Role::System) {
            thread.add_message(Message::system(SYSTEM_PROMPT));
        }

        let req = self.build_request(thread);
        let tool_defs = self.tools.tool_defs_for_model(true);
        let req = if let Some(tools) = tool_defs {
            req.with_tools(tools)
        } else {
            req
        };

        self.provider.complete_stream(&req).await
    }

    fn build_request(&self, _thread: &AgentThread) -> CompletionRequest {
        CompletionRequest::new(&self.config.agent.default_model)
            .with_system(SYSTEM_PROMPT)
    }
}

#[derive(Debug, Clone)]
pub enum AgentOutput {
    Success { text: String },
    Error { message: String },
}

impl AgentOutput {
    pub fn success(text: impl Into<String>) -> Self {
        Self::Success { text: text.into() }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error { message: message.into() }
    }
}

pub type AgentResult = Result<AgentOutput, AgentError>;
pub type AgentOutputStream = Box<dyn tokio_stream::Stream<Item = Result<sentinel_protocol::StreamChunk, ProviderError>> + Send + Unpin>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("Agent error: {0}")]
    Generic(String),
}
