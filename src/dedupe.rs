//! Safety-gated deletion / relocation of exact duplicates.
//!
//! Perceptual ("visually similar") groups are never eligible here — only files
//! with identical content can be safely removed automatically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::app::confirm;
use crate::error::{AppError, Result};
use crate::scanner::ScanResults;

enum Action {
    Delete,
    Move(PathBuf),
}

/// Interactively act on the duplicate groups in `results`.
pub fn run(results: &ScanResults) -> Result<()> {
    if !results.is_exact() {
        println!(
            "{} Deletion is disabled for visually-similar groups (they are not identical).",
            style("ℹ").cyan()
        );
        return Ok(());
    }
    if results.groups.is_empty() {
        return Ok(());
    }

    let theme = ColorfulTheme::default();

    let action = match Select::with_theme(&theme)
        .with_prompt("What should happen to the duplicates?")
        .items(&[
            "Delete duplicates (keep one per group)",
            "Move duplicates to a folder",
            "Cancel",
        ])
        .default(2)
        .interact()?
    {
        0 => Action::Delete,
        1 => {
            let dest: String = Input::with_theme(&theme)
                .with_prompt("Destination folder for duplicates")
                .interact_text()?;
            Action::Move(PathBuf::from(dest))
        }
        _ => {
            println!("Cancelled. No files changed.");
            return Ok(());
        }
    };

    let manual = Select::with_theme(&theme)
        .with_prompt("Which copy should be kept in each group?")
        .items(&[
            "Keep the shortest path (automatic)",
            "Choose manually per group",
        ])
        .default(0)
        .interact()?
        == 1;

    // Build the plan: for each group, one keeper and the victims to act on.
    let mut plan: Vec<(PathBuf, Vec<PathBuf>)> = Vec::new();
    for g in &results.groups {
        let paths: Vec<PathBuf> = g.files.iter().map(|f| f.path.clone()).collect();
        let keeper_idx = if manual {
            let labels: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            Select::with_theme(&theme)
                .with_prompt(format!("Group '{}' — keep which file?", g.key))
                .items(&labels)
                .default(shortest_idx(&paths))
                .interact()?
        } else {
            shortest_idx(&paths)
        };
        let victims = paths
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != keeper_idx)
            .map(|(_, p)| p.clone())
            .collect();
        plan.push((paths[keeper_idx].clone(), victims));
    }

    // Dry-run preview — always shown before anything is touched.
    println!("\n{} Dry-run preview:", style("▸").cyan().bold());
    let mut total = 0u64;
    for (keeper, victims) in &plan {
        println!("  {} {}", style("keep  ").green(), keeper.display());
        for v in victims {
            let verb = match action {
                Action::Delete => "delete",
                Action::Move(_) => "move  ",
            };
            println!("  {} {}", style(verb).red(), v.display());
            total += 1;
        }
    }
    println!();

    if total == 0 {
        println!("Nothing to do.");
        return Ok(());
    }

    let confirmed = confirm(
        &theme,
        &format!(
            "Apply these changes to {} file(s)? This cannot be undone.",
            total
        ),
        false,
    )?;
    if !confirmed {
        println!("Cancelled. No files changed.");
        return Ok(());
    }

    if let Action::Move(ref dest) = action {
        fs::create_dir_all(dest).map_err(|e| AppError::Io(dest.clone(), e))?;
    }

    let mut done = 0u64;
    let mut failed = 0u64;
    for (_, victims) in &plan {
        for v in victims {
            let res = match &action {
                Action::Delete => fs::remove_file(v),
                Action::Move(dest) => move_file(v, dest),
            };
            match res {
                Ok(_) => done += 1,
                Err(e) => {
                    failed += 1;
                    eprintln!("{} failed on {}: {}", style("⚠").yellow(), v.display(), e);
                }
            }
        }
    }

    println!(
        "{} {} file(s) processed, {} failed.",
        style("✅").green(),
        done,
        failed
    );
    Ok(())
}

/// Index of the path with the fewest characters (a stable "keep this" heuristic).
fn shortest_idx(paths: &[PathBuf]) -> usize {
    paths
        .iter()
        .enumerate()
        .min_by_key(|(_, p)| p.as_os_str().len())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Move `src` into `dest_dir`, avoiding name collisions; falls back to
/// copy+remove across filesystem boundaries.
fn move_file(src: &Path, dest_dir: &Path) -> io::Result<()> {
    let name = src.file_name().unwrap_or_default();
    let mut target = dest_dir.join(name);
    let mut n = 1;
    while target.exists() {
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        target = dest_dir.join(format!("{}_{}{}", stem, n, ext));
        n += 1;
    }
    fs::rename(src, &target).or_else(|_| {
        fs::copy(src, &target)?;
        fs::remove_file(src)
    })
}
