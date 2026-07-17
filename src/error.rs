//! Error types for DupFile-Analyzer.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// All fallible operations in the crate return this error.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error accessing '{0}': {1}")]
    Io(PathBuf, #[source] io::Error),

    #[error("No files found in directory '{0}'")]
    EmptyDirectory(PathBuf),

    #[error("'{0}' is not a valid directory")]
    NotADirectory(PathBuf),

    #[error("Failed to decode image '{0}': {1}")]
    Image(PathBuf, #[source] image::ImageError),

    #[error("Invalid glob pattern '{0}': {1}")]
    Glob(String, #[source] globset::Error),

    #[error("Interactive prompt failed: {0}")]
    Prompt(#[from] dialoguer::Error),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, AppError>;
