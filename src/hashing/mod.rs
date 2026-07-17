//! Hashing algorithms and shared helpers.

pub mod content;
pub mod perceptual;

use std::path::Path;

/// Perceptual hashing algorithms available for image similarity scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    AHash,
    DHash,
    PHash,
}

impl Algorithm {
    /// All algorithms, in menu order.
    pub const ALL: [Algorithm; 3] = [Algorithm::AHash, Algorithm::DHash, Algorithm::PHash];

    /// Short human label.
    pub fn label(&self) -> &'static str {
        match self {
            Algorithm::AHash => "aHash (average hash)",
            Algorithm::DHash => "dHash (difference hash)",
            Algorithm::PHash => "pHash (DCT hash)",
        }
    }

    /// One-line description shown in the wizard.
    pub fn description(&self) -> &'static str {
        match self {
            Algorithm::AHash => "Fastest. Compares each pixel to the average brightness.",
            Algorithm::DHash => "Balanced. Encodes brightness gradients between neighbors.",
            Algorithm::PHash => "Most robust. Uses the DCT low-frequency signature.",
        }
    }

    /// Compute the 64-bit perceptual hash of an image with this algorithm.
    pub fn compute(&self, path: &Path) -> crate::error::Result<u64> {
        match self {
            Algorithm::AHash => perceptual::ahash(path),
            Algorithm::DHash => perceptual::dhash(path),
            Algorithm::PHash => perceptual::phash(path),
        }
    }
}

/// File extensions treated as images (lowercase, no dot).
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif",
];

/// Whether a path has a recognized image extension.
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
