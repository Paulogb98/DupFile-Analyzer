//! Perceptual image hashes: aHash, dHash and pHash.
//!
//! Unlike a cryptographic hash, a perceptual hash produces *similar* outputs for
//! *visually similar* images. Two images are considered near-duplicates when the
//! Hamming distance between their 64-bit hashes is small.

use std::path::Path;

use image::imageops::FilterType;

use crate::error::{AppError, Result};

/// Load an image, convert to grayscale and resize to `w`x`h`, returning the
/// luminance of every pixel in row-major order as `f64`.
fn load_luma(path: &Path, w: u32, h: u32) -> Result<Vec<f64>> {
    let img = image::open(path).map_err(|e| AppError::Image(path.to_path_buf(), e))?;
    let gray = img
        .grayscale()
        .resize_exact(w, h, FilterType::Triangle)
        .to_luma8();
    Ok(gray.pixels().map(|p| p[0] as f64).collect())
}

/// Average hash: resize to 8x8, set each bit when the pixel exceeds the mean.
///
/// Fastest and simplest; robust to scaling and mild color shifts, but sensitive
/// to changes in overall brightness/contrast.
pub fn ahash(path: &Path) -> Result<u64> {
    let px = load_luma(path, 8, 8)?;
    let mean = px.iter().sum::<f64>() / px.len() as f64;
    let mut hash = 0u64;
    for (i, &v) in px.iter().enumerate() {
        if v > mean {
            hash |= 1 << i;
        }
    }
    Ok(hash)
}

/// Difference hash: resize to 9x8, set each bit when a pixel is brighter than its
/// right-hand neighbor (8 comparisons per row, 64 bits total).
///
/// Encodes gradients rather than absolute brightness, so it tolerates global
/// brightness/contrast changes better than aHash while staying cheap.
pub fn dhash(path: &Path) -> Result<u64> {
    const W: usize = 9;
    const H: usize = 8;
    let px = load_luma(path, W as u32, H as u32)?;
    let mut hash = 0u64;
    let mut bit = 0;
    for row in 0..H {
        for col in 0..(W - 1) {
            let left = px[row * W + col];
            let right = px[row * W + col + 1];
            if left > right {
                hash |= 1 << bit;
            }
            bit += 1;
        }
    }
    Ok(hash)
}

/// Perceptual hash: resize to 32x32, run a 2D DCT-II, keep the top-left 8x8
/// low-frequency block, and set each bit when a coefficient exceeds the median.
///
/// The most robust of the three against resizing, compression and small edits,
/// at a higher compute cost (the DCT).
pub fn phash(path: &Path) -> Result<u64> {
    const SIZE: usize = 32;
    let px = load_luma(path, SIZE as u32, SIZE as u32)?;

    // DCT over each row.
    let mut rows = vec![0.0; SIZE * SIZE];
    for r in 0..SIZE {
        let row_dct = dct_1d(&px[r * SIZE..(r + 1) * SIZE]);
        rows[r * SIZE..(r + 1) * SIZE].copy_from_slice(&row_dct);
    }

    // DCT over each column of the row-transformed data.
    let mut dct = vec![0.0; SIZE * SIZE];
    let mut column = vec![0.0; SIZE];
    for c in 0..SIZE {
        for r in 0..SIZE {
            column[r] = rows[r * SIZE + c];
        }
        let col_dct = dct_1d(&column);
        for r in 0..SIZE {
            dct[r * SIZE + c] = col_dct[r];
        }
    }

    // Keep the top-left 8x8 block of low-frequency coefficients.
    let mut block = Vec::with_capacity(64);
    for r in 0..8 {
        for c in 0..8 {
            block.push(dct[r * SIZE + c]);
        }
    }

    // Median of the block, excluding the DC term (index 0) which dominates.
    let mut sorted: Vec<f64> = block[1..].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];

    let mut hash = 0u64;
    for (i, &v) in block.iter().enumerate() {
        if v > median {
            hash |= 1 << i;
        }
    }
    Ok(hash)
}

/// One-dimensional DCT-II (unnormalized — only relative ordering matters here).
fn dct_1d(input: &[f64]) -> Vec<f64> {
    let n = input.len();
    let mut out = vec![0.0; n];
    for (u, slot) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (x, &val) in input.iter().enumerate() {
            sum += val
                * ((std::f64::consts::PI * (2.0 * x as f64 + 1.0) * u as f64) / (2.0 * n as f64))
                    .cos();
        }
        *slot = sum;
    }
    out
}

/// Hamming distance between two hashes — the number of differing bits.
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}
