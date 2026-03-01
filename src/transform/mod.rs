// Transformation tools module for Store manipulation.

pub mod align;
pub mod chunk;
pub mod discard;
pub mod flatten;
pub mod flatten_regex;

pub use discard::{discard_groups_from_file, DiscardResult};
pub use flatten::{flatten_keep_from_file, FlattenKeepResult};
pub use flatten_regex::{flatten_regex, FlattenRegexResult};

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),

    #[error("Invalid tag file: {0}")]
    InvalidTagFile(String),
}

pub(crate) fn parse_tag_file(path: &Path) -> Result<Vec<String>, TransformError> {
    let content = std::fs::read_to_string(path)?;
    let mut tags = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let tag = line.rsplit('\t').next().unwrap_or(line).trim().to_string();
        if !tag.is_empty() {
            tags.push(tag);
        }
    }

    if tags.is_empty() {
        return Err(TransformError::InvalidTagFile(format!(
            "no valid tags found in {}",
            path.display()
        )));
    }

    Ok(tags)
}
