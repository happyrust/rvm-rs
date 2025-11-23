// Geometry processing module for connection detection and alignment
// This module provides functionality to detect connections between adjacent geometries
// and align their sampling points for better mesh quality

pub mod anchor;
pub mod connection;

pub use anchor::Anchor;
pub use connection::ConnectionDetector;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Invalid geometry type for anchor generation: {0}")]
    InvalidGeometryType(String),

    #[error("Transform matrix is not invertible")]
    NonInvertibleTransform,

    #[error("Connection detection failed: {0}")]
    ConnectionDetectionFailed(String),

    #[error("Alignment failed: {0}")]
    AlignmentFailed(String),
}

