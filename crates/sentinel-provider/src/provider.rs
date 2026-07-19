use async_trait::async_trait;
use sentinel_protocol::{CompletionRequest, CompletionResponse, StreamChunk, ToolDef};
use sentinel_provider_info::ProviderInfo;
use crate::error::ProviderError;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn info(&self) -> &ProviderInfo;
    fn name(&self) -> &str { self.info().name.as_str() }

    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, ProviderError>;
    async fn complete_stream(&self, req: &CompletionRequest) -> Result<Box<dyn tokio_stream::Stream<Item = Result<StreamChunk, ProviderError>> + Send + Unpin>, ProviderError>;

    fn supports_tool(&self, tool: &ToolDef) -> bool {
        self.info().models.iter().any(|m| m.supports_tools && m.id == tool.name)
    }
}
