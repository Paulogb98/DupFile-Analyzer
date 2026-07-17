//! Human-readable reporting of scan results.

use console::Style;

use crate::scanner::ScanResults;

/// Format a byte count into a compact human-readable string.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

/// Print a full report of a scan to stdout.
pub fn print_report(results: &ScanResults) {
    let title = Style::new().cyan().bold();
    let key_style = Style::new().yellow().bold();
    let file_style = Style::new().green();
    let dim = Style::new().dim();

    for e in &results.errors {
        eprintln!("{} {}", Style::new().yellow().apply_to("⚠"), e);
    }

    if results.groups.is_empty() {
        println!(
            "{} No duplicates found.",
            Style::new().green().bold().apply_to("✅")
        );
        return;
    }

    if results.is_exact() {
        println!(
            "{} Found {} duplicate group(s).\n",
            title.apply_to("ℹ"),
            results.groups.len()
        );
        for g in &results.groups {
            let size = g.files.first().map(|f| f.size).unwrap_or(0);
            println!(
                "{} {}  {}",
                key_style.apply_to("Hash:"),
                g.key,
                dim.apply_to(format!("({} each)", human_bytes(size)))
            );
            for f in &g.files {
                println!("  - {}", file_style.apply_to(f.path.display()));
            }
            println!();
        }
        println!(
            "{} {} duplicate file(s) across {} group(s) — {} reclaimable.",
            title.apply_to("▸"),
            results.duplicate_files(),
            results.groups.len(),
            human_bytes(results.reclaimable_bytes()),
        );
    } else {
        let algo = results.algorithm.map(|a| a.label()).unwrap_or("perceptual");
        let threshold = results.threshold.unwrap_or(0);
        println!(
            "{} Found {} group(s) of visually similar images ({}, threshold {}).\n",
            title.apply_to("ℹ"),
            results.groups.len(),
            algo,
            threshold
        );
        for g in &results.groups {
            println!("{} {}", key_style.apply_to("Similar:"), g.key);
            for f in &g.files {
                println!("  - {}", file_style.apply_to(f.path.display()));
            }
            println!();
        }
        println!(
            "{} {} similar image(s) across {} group(s).",
            title.apply_to("▸"),
            results.duplicate_files(),
            results.groups.len(),
        );
    }
}
