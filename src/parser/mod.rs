pub mod att;
pub mod common;
pub mod rvm;

pub use att::parse_att;
pub use rvm::parse_rvm;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid chunk header at offset {offset:#x}: expected {expected}, found {found}")]
    InvalidChunkHeader {
        offset: usize,
        expected: String,
        found: String,
    },

    #[error("Unexpected end of file at offset {offset:#x}")]
    UnexpectedEof { offset: usize },

    #[error("Invalid geometry kind: {kind}")]
    InvalidGeometryKind { kind: u32 },

    #[error("Nom parsing error: {0}")]
    NomError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Attribute parsing error: {0}")]
    AttributeError(String),
}

impl<I: std::fmt::Debug> From<nom::Err<nom::error::Error<I>>> for ParseError {
    fn from(err: nom::Err<nom::error::Error<I>>) -> Self {
        ParseError::NomError(format!("{:?}", err))
    }
}
