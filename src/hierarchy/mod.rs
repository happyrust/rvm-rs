// Hierarchy tools module for scene graph manipulation
// Provides functionality for flattening, filtering, and simplifying scene graphs

pub mod discard;
pub mod flatten_keep;
pub mod flatten_regex;

pub use discard::GroupDiscarder;
pub use flatten_keep::KeepFlattener;
pub use flatten_regex::RegexFlattener;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HierarchyError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid tag file format: {0}")]
    InvalidTagFile(String),

    #[error("Node not found: {0:?}")]
    NodeNotFound(String),

    #[error("Circular reference detected")]
    CircularReference,
}

