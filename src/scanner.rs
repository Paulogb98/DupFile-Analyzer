//! Directory scanning: collection, filtering, hashing and grouping.

use std::collections::HashMap;
use std::path::PathBuf;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::{DirEntry, WalkDir};

use crate::config::{ScanConfig, ScanMode};
use crate::error::{AppError, Result};
use crate::hashing::{content, is_image, perceptual, Algorithm};

/// A single file inside a duplicate group.
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// A set of files that hash to the same key (exact) or cluster together (similar).
pub struct DuplicateGroup {
    pub key: String,
    pub files: Vec<FileEntry>,
}

/// The outcome of a scan.
pub struct ScanResults {
    pub mode: ScanMode,
    pub algorithm: Option<Algorithm>,
    pub threshold: Option<u32>,
    pub scanned: usize,
    pub errors: Vec<String>,
    pub groups: Vec<DuplicateGroup>,
}

impl ScanResults {
    /// Exact-content modes support wasted-space math and deletion; perceptual does not.
    pub fn is_exact(&self) -> bool {
        self.mode != ScanMode::SimilarImages
    }

    /// Number of redundant files (each group keeps one, the rest are duplicates).
    pub fn duplicate_files(&self) -> usize {
        self.groups
            .iter()
            .map(|g| g.files.len().saturating_sub(1))
            .sum()
    }

    /// Bytes that could be reclaimed by keeping one file per exact group.
    pub fn reclaimable_bytes(&self) -> u64 {
        if !self.is_exact() {
            return 0;
        }
        self.groups
            .iter()
            .map(|g| {
                let size = g.files.first().map(|f| f.size).unwrap_or(0);
                size * (g.files.len() as u64 - 1)
            })
            .sum()
    }
}

/// Run a scan according to `cfg`.
pub fn scan(cfg: &ScanConfig) -> Result<ScanResults> {
    let files = collect_files(cfg);
    if files.is_empty() {
        return Err(AppError::EmptyDirectory(cfg.root.clone()));
    }

    println!(
        "{} {} file(s) to analyze.\n",
        style("✔").green(),
        files.len()
    );
    let scanned = files.len();

    match cfg.mode {
        ScanMode::SimilarImages => scan_perceptual(cfg, files, scanned),
        _ => scan_exact(cfg, files, scanned),
    }
}

/// Walk the tree and apply all filters, returning `(path, size)` pairs.
fn collect_files(cfg: &ScanConfig) -> Vec<(PathBuf, u64)> {
    WalkDir::new(&cfg.root)
        .follow_links(cfg.follow_symlinks)
        .into_iter()
        .filter_entry(|e| cfg.include_hidden || !is_hidden(e))
        .filter_map(|res| res.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let path = e.path().to_path_buf();

            // Image modes ignore non-image files.
            if cfg.mode != ScanMode::ExactFiles && !is_image(&path) {
                return None;
            }

            let size = e.metadata().ok()?.len();
            if let Some(min) = cfg.min_size {
                if size < min {
                    return None;
                }
            }
            if let Some(max) = cfg.max_size {
                if size > max {
                    return None;
                }
            }
            if let Some(ref exclude) = cfg.exclude {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if exclude.is_match(name) {
                        return None;
                    }
                }
            }

            Some((path, size))
        })
        .collect()
}

/// Dot-prefixed entries are treated as hidden (portable approximation).
fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() != 0
        && entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
}

fn progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb
}

/// Exact-content scan: size pre-filter, then partial then full SHA-256.
fn scan_exact(
    cfg: &ScanConfig,
    files: Vec<(PathBuf, u64)>,
    scanned: usize,
) -> Result<ScanResults> {
    let mut errors = Vec::new();

    // Stage 1: only files whose size collides with another can be duplicates.
    let mut by_size: HashMap<u64, Vec<(PathBuf, u64)>> = HashMap::new();
    for (path, size) in files {
        by_size.entry(size).or_default().push((path, size));
    }
    let candidates: Vec<(PathBuf, u64)> = by_size
        .into_values()
        .filter(|v| v.len() > 1)
        .flatten()
        .collect();

    let mut groups = Vec::new();
    if !candidates.is_empty() {
        // Stage 2: cheap partial hash to weed out non-matches.
        let pb = progress_bar(candidates.len() as u64);
        let partial: Vec<_> = candidates
            .par_iter()
            .map(|(path, size)| {
                let res = content::partial_hash(path).map(|h| (h, path.clone(), *size));
                pb.inc(1);
                res
            })
            .collect();
        pb.finish_and_clear();

        let mut by_partial: HashMap<(u64, String), Vec<(PathBuf, u64)>> = HashMap::new();
        for res in partial {
            match res {
                Ok((hash, path, size)) => {
                    by_partial.entry((size, hash)).or_default().push((path, size))
                }
                Err(e) => errors.push(e.to_string()),
            }
        }
        let full_candidates: Vec<(PathBuf, u64)> = by_partial
            .into_values()
            .filter(|v| v.len() > 1)
            .flatten()
            .collect();

        // Stage 3: full SHA-256 confirms true duplicates.
        if !full_candidates.is_empty() {
            let pb = progress_bar(full_candidates.len() as u64);
            let full: Vec<_> = full_candidates
                .par_iter()
                .map(|(path, size)| {
                    let res = content::full_hash(path).map(|h| (h, path.clone(), *size));
                    pb.inc(1);
                    res
                })
                .collect();
            pb.finish_and_clear();

            let mut map: HashMap<String, Vec<FileEntry>> = HashMap::new();
            for res in full {
                match res {
                    Ok((hash, path, size)) => {
                        map.entry(hash).or_default().push(FileEntry { path, size })
                    }
                    Err(e) => errors.push(e.to_string()),
                }
            }
            groups = map
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(key, files)| DuplicateGroup { key, files })
                .collect();
        }
    }

    // Biggest space savings first.
    groups.sort_by_key(|g| {
        let size = g.files.first().map(|f| f.size).unwrap_or(0);
        std::cmp::Reverse(size * g.files.len() as u64)
    });

    Ok(ScanResults {
        mode: cfg.mode,
        algorithm: None,
        threshold: None,
        scanned,
        errors,
        groups,
    })
}

/// Perceptual scan: hash every image, then cluster by Hamming distance.
fn scan_perceptual(
    cfg: &ScanConfig,
    files: Vec<(PathBuf, u64)>,
    scanned: usize,
) -> Result<ScanResults> {
    let algorithm = cfg.algorithm;

    let pb = progress_bar(files.len() as u64);
    let hashed: Vec<_> = files
        .par_iter()
        .map(|(path, size)| {
            let res = algorithm.compute(path).map(|h| (h, path.clone(), *size));
            pb.inc(1);
            res
        })
        .collect();
    pb.finish_and_clear();

    let mut items: Vec<(u64, PathBuf, u64)> = Vec::new();
    let mut errors = Vec::new();
    for res in hashed {
        match res {
            Ok(item) => items.push(item),
            Err(e) => errors.push(e.to_string()),
        }
    }

    // Cluster with union-find: any pair within the threshold joins the same group.
    let n = items.len();
    let mut uf = UnionFind::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            if perceptual::hamming(items[i].0, items[j].0) <= cfg.threshold {
                uf.union(i, j);
            }
        }
    }

    let mut clusters: HashMap<usize, Vec<FileEntry>> = HashMap::new();
    for (i, (_, path, size)) in items.into_iter().enumerate() {
        let root = uf.find(i);
        clusters
            .entry(root)
            .or_default()
            .push(FileEntry { path, size });
    }

    let mut groups: Vec<DuplicateGroup> = clusters
        .into_values()
        .filter(|v| v.len() > 1)
        .enumerate()
        .map(|(idx, files)| DuplicateGroup {
            key: format!("cluster {}", idx + 1),
            files,
        })
        .collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.files.len()));

    Ok(ScanResults {
        mode: cfg.mode,
        algorithm: Some(algorithm),
        threshold: Some(cfg.threshold),
        scanned,
        errors,
        groups,
    })
}

/// Disjoint-set union with path compression and union by rank.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            std::cmp::Ordering::Less => self.parent[ra] = rb,
            std::cmp::Ordering::Greater => self.parent[rb] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
    }
}
