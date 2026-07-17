//! Integration tests for the scanning pipeline.

use std::fs;
use std::path::PathBuf;

use dupfile_analyzer::config::{ScanConfig, ScanMode};
use dupfile_analyzer::scanner::scan;
use image::{Rgb, RgbImage};

/// Create a unique temporary directory for a test.
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "dfa-test-{}-{}",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 8 vertical stripes — brightness varies horizontally (rich dHash signature).
fn vertical_stripes(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, _| {
        let v = if (x * 8 / w.max(1)).is_multiple_of(2) { 20 } else { 235 };
        Rgb([v, v, v])
    })
}

/// 8 horizontal stripes — flat within each row (distinct from vertical stripes).
fn horizontal_stripes(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |_, y| {
        let v = if (y * 8 / h.max(1)).is_multiple_of(2) { 20 } else { 235 };
        Rgb([v, v, v])
    })
}

#[test]
fn exact_duplicates_are_grouped_and_uniques_ignored() {
    let dir = temp_dir("exact");
    fs::write(dir.join("a.txt"), b"hello world duplicate content").unwrap();
    fs::write(dir.join("b.txt"), b"hello world duplicate content").unwrap();
    fs::write(dir.join("c.txt"), b"totally different content here").unwrap();

    let cfg = ScanConfig::new(dir.clone(), ScanMode::ExactFiles);
    let results = scan(&cfg).unwrap();

    assert_eq!(results.groups.len(), 1, "exactly one duplicate group");
    assert_eq!(results.groups[0].files.len(), 2, "two identical files");
    assert_eq!(results.duplicate_files(), 1, "one redundant file");
    assert!(results.reclaimable_bytes() > 0);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn min_size_filter_excludes_small_files() {
    let dir = temp_dir("filter");
    fs::write(dir.join("small_a.bin"), b"tiny").unwrap();
    fs::write(dir.join("small_b.bin"), b"tiny").unwrap();

    let mut cfg = ScanConfig::new(dir.clone(), ScanMode::ExactFiles);
    cfg.min_size = Some(1024); // 1 KB — excludes the tiny files
    // No files survive the filter -> EmptyDirectory.
    match scan(&cfg) {
        Err(dupfile_analyzer::AppError::EmptyDirectory(_)) => {}
        other => panic!("expected EmptyDirectory, got {:?}", other.is_ok()),
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn perceptual_clusters_similar_images() {
    let dir = temp_dir("phash");

    // A and B: same vertical-stripe pattern at different resolutions (visually alike).
    vertical_stripes(256, 256)
        .save(dir.join("a.png"))
        .unwrap();
    vertical_stripes(200, 180)
        .save(dir.join("b.png"))
        .unwrap();
    // C: horizontal stripes (visually different).
    horizontal_stripes(256, 256)
        .save(dir.join("c.png"))
        .unwrap();

    let mut cfg = ScanConfig::new(dir.clone(), ScanMode::SimilarImages);
    cfg.threshold = 12;
    let results = scan(&cfg).unwrap();

    assert_eq!(results.groups.len(), 1, "one cluster of similar images");
    let group = &results.groups[0];
    let names: Vec<String> = group
        .files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"a.png".to_string()));
    assert!(names.contains(&"b.png".to_string()));
    assert!(!names.contains(&"c.png".to_string()), "vertical gradient must not cluster");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn image_modes_ignore_non_images() {
    let dir = temp_dir("imgonly");
    fs::write(dir.join("notes.txt"), b"same").unwrap();
    fs::write(dir.join("notes2.txt"), b"same").unwrap();

    let cfg = ScanConfig::new(dir.clone(), ScanMode::ExactImages);
    // No images present -> empty.
    assert!(scan(&cfg).is_err());

    fs::remove_dir_all(&dir).ok();
}
