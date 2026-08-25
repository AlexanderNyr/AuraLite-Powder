use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Version mismatch: file version {file} > current {current}")]
    VersionMismatch { file: u32, current: u32 },
    #[error("Compression error: {0}")]
    Compression(String),
    #[error("Invalid file format")]
    InvalidFormat,
    #[error("Save grid too large: {width}x{height} (max side 8192, max area 16777216)")]
    GridTooLarge { width: u32, height: u32 },
}
