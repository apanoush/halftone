//! Error type shared by the engine.

/// Engine error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("failed to load OBJ mesh: {0}")]
    Obj(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("invalid settings: {0}")]
    Settings(String),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;
