use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Empty stream chunk")]
    EmptyStreamChunk,
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Unexpected content block type")]
    UnexpectedContentBlock,
}
