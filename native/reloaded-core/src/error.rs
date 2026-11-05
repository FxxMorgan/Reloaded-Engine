use thiserror::Error;

/// Core error types for the Reloaded engine.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid package: {0}")]
    InvalidPackage(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Save error: {0}")]
    Save(String),

    #[error("Renderer error: {0}")]
    Renderer(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Invalid state transition: {0}")]
    InvalidState(String),
}
