//! Scan configuration built by the interactive wizard.

use std::path::PathBuf;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::{AppError, Result};
use crate::hashing::Algorithm;

/// What the user wants to look for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    /// All files, matched by exact SHA-256 content hash.
    ExactFiles,
    /// Image files only, matched by exact SHA-256 content hash.
    ExactImages,
    /// Image files, matched by perceptual similarity.
    SimilarImages,
}

/// Everything the scanner needs for one run.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub root: PathBuf,
    pub mode: ScanMode,
    /// Perceptual algorithm (only used when `mode == SimilarImages`).
    pub algorithm: Algorithm,
    /// Max Hamming distance to treat two images as similar.
    pub threshold: u32,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub exclude: Option<GlobSet>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
}

impl ScanConfig {
    /// A config with sensible defaults for the given root and mode.
    pub fn new(root: PathBuf, mode: ScanMode) -> Self {
        Self {
            root,
            mode,
            algorithm: Algorithm::DHash,
            threshold: 10,
            min_size: None,
            max_size: None,
            exclude: None,
            include_hidden: false,
            follow_symlinks: false,
        }
    }
}

/// Build a [`GlobSet`] from filename patterns (e.g. `*.tmp`, `cache_*`).
pub fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).map_err(|e| AppError::Glob(p.clone(), e))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| AppError::Glob("<pattern set>".to_string(), e))
}
