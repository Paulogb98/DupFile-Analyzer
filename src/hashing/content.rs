//! Exact content hashing (SHA-256) with a cheap partial pre-hash.
//!
//! Two-stage strategy: [`partial_hash`] reads only the first few KB to quickly
//! separate obvious non-matches, and [`full_hash`] computes the complete SHA-256
//! only for files that survive the size + partial-hash filters.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

/// Read buffer size for full hashing (64 KB — a safe, efficient value).
const BUFFER_SIZE: usize = 65536;

/// Number of leading bytes used for the quick partial pre-hash.
const PARTIAL_SIZE: usize = 8192; // 8 KB

/// Compute the full SHA-256 hash of a file, streamed through a 64 KB buffer.
pub fn full_hash(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| AppError::Io(path.to_path_buf(), e))?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| AppError::Io(path.to_path_buf(), e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute a SHA-256 over only the first [`PARTIAL_SIZE`] bytes of a file.
///
/// Used as a cheap discriminator: files whose partial hashes differ cannot be
/// identical, so they never need a full read.
pub fn partial_hash(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| AppError::Io(path.to_path_buf(), e))?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; PARTIAL_SIZE];
    let mut filled = 0;

    while filled < PARTIAL_SIZE {
        let bytes_read = reader
            .read(&mut buffer[filled..])
            .map_err(|e| AppError::Io(path.to_path_buf(), e))?;
        if bytes_read == 0 {
            break;
        }
        filled += bytes_read;
    }

    let mut hasher = Sha256::new();
    hasher.update(&buffer[..filled]);
    Ok(format!("{:x}", hasher.finalize()))
}
