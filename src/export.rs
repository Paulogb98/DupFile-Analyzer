//! Export scan results to JSON, CSV or plain text.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::Serialize;

use crate::error::{AppError, Result};
use crate::scanner::ScanResults;

/// Output format for an export.
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Json,
    Csv,
    Text,
}

impl Format {
    /// Default file extension for the format.
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Csv => "csv",
            Format::Text => "txt",
        }
    }
}

#[derive(Serialize)]
struct ExportGroup {
    key: String,
    file_size: u64,
    files: Vec<String>,
}

#[derive(Serialize)]
struct ExportDoc {
    mode: String,
    algorithm: Option<String>,
    threshold: Option<u32>,
    scanned: usize,
    duplicate_files: usize,
    reclaimable_bytes: u64,
    groups: Vec<ExportGroup>,
}

fn to_doc(r: &ScanResults) -> ExportDoc {
    ExportDoc {
        mode: format!("{:?}", r.mode),
        algorithm: r.algorithm.map(|a| a.label().to_string()),
        threshold: r.threshold,
        scanned: r.scanned,
        duplicate_files: r.duplicate_files(),
        reclaimable_bytes: r.reclaimable_bytes(),
        groups: r
            .groups
            .iter()
            .map(|g| ExportGroup {
                key: g.key.clone(),
                file_size: g.files.first().map(|f| f.size).unwrap_or(0),
                files: g.files.iter().map(|f| f.path.display().to_string()).collect(),
            })
            .collect(),
    }
}

/// Serialize `results` in `format` and write it to `path`.
pub fn export(results: &ScanResults, format: Format, path: &Path) -> Result<()> {
    let content = match format {
        Format::Json => {
            serde_json::to_string_pretty(&to_doc(results)).expect("results serialize to JSON")
        }
        Format::Csv => to_csv(results),
        Format::Text => to_text(results),
    };

    let mut file = File::create(path).map_err(|e| AppError::Io(path.to_path_buf(), e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| AppError::Io(path.to_path_buf(), e))?;
    Ok(())
}

fn to_csv(r: &ScanResults) -> String {
    let mut out = String::from("group,file,size_bytes\n");
    for g in &r.groups {
        for f in &g.files {
            out.push_str(&format!(
                "{},{},{}\n",
                csv_escape(&g.key),
                csv_escape(&f.path.display().to_string()),
                f.size
            ));
        }
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn to_text(r: &ScanResults) -> String {
    let mut out = String::new();
    for g in &r.groups {
        out.push_str(&format!("{}\n", g.key));
        for f in &g.files {
            out.push_str(&format!("  - {}\n", f.path.display()));
        }
        out.push('\n');
    }
    out
}
