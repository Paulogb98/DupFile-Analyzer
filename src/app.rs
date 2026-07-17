//! The interactive terminal wizard.

use std::env;
use std::path::PathBuf;

use console::{measure_text_width, Style};
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::config::{build_globset, ScanConfig, ScanMode};
use crate::dedupe;
use crate::error::{AppError, Result};
use crate::export::{self, Format};
use crate::hashing::Algorithm;
use crate::report;
use crate::scanner::{self, ScanResults};

/// Entry point for the wizard loop.
pub fn run() -> Result<()> {
    print_banner();
    let theme = ColorfulTheme::default();

    loop {
        let cfg = build_config(&theme)?;
        println!();

        match scanner::scan(&cfg) {
            Ok(results) => {
                report::print_report(&results);
                post_scan(&theme, &results)?;
            }
            Err(AppError::EmptyDirectory(p)) => {
                println!(
                    "{} No matching files found in {}.",
                    Style::new().yellow().bold().apply_to("⚠"),
                    p.display()
                );
            }
            Err(e) => return Err(e),
        }

        println!();
        if !confirm(&theme, "Scan another folder?", false)? {
            break;
        }
        println!();
    }

    println!(
        "\n{} Done. Happy deduping!",
        Style::new().cyan().bold().apply_to("👋")
    );
    Ok(())
}

/// Arrow-key Yes/No prompt (dialoguer's `Confirm` only accepts y/n keystrokes).
pub(crate) fn confirm(theme: &ColorfulTheme, prompt: &str, default: bool) -> Result<bool> {
    let idx = Select::with_theme(theme)
        .with_prompt(prompt)
        .items(&["Yes", "No"])
        .default(if default { 0 } else { 1 })
        .interact()?;
    Ok(idx == 0)
}

fn print_banner() {
    let border = Style::new().cyan().bold();
    let dim = Style::new().dim();
    let version = env!("CARGO_PKG_VERSION");

    let title = format!("       🔍  DupFile-Analyzer  v{}", version);
    let subtitle = "Find duplicate files & visually similar images";
    let content_width = measure_text_width(&title).max(measure_text_width(subtitle));

    println!();
    println!(
        "  {}",
        border.apply_to(format!("╭{}╮", "─".repeat(content_width + 2)))
    );
    println!("  {}", boxed_line(&border, &title, content_width));
    println!("  {}", boxed_line(&dim, subtitle, content_width));
    println!(
        "  {}",
        border.apply_to(format!("╰{}╯", "─".repeat(content_width + 2)))
    );
    println!();
    println!(
        "  {}",
        dim.apply_to("Use ↑/↓ to navigate, Enter to select.")
    );
    println!();
}

/// Render one line of the banner box, padded so the right border lines up
/// regardless of the visible width of `text` (emoji count as 2 columns).
fn boxed_line(text_style: &Style, text: &str, content_width: usize) -> String {
    let border = Style::new().cyan().bold();
    let pad = " ".repeat(content_width.saturating_sub(measure_text_width(text)));
    format!(
        "{} {}{} {}",
        border.apply_to("│"),
        text_style.apply_to(text),
        pad,
        border.apply_to("│")
    )
}

/// Drive the prompts that assemble a [`ScanConfig`].
fn build_config(theme: &ColorfulTheme) -> Result<ScanConfig> {
    let mode = match Select::with_theme(theme)
        .with_prompt("What do you want to scan?")
        .items(&[
            "General files — exact duplicates (SHA-256)",
            "Images — exact duplicates (SHA-256)",
            "Images — visually similar (perceptual hash)",
        ])
        .default(0)
        .interact()?
    {
        0 => ScanMode::ExactFiles,
        1 => ScanMode::ExactImages,
        _ => ScanMode::SimilarImages,
    };

    let default_dir = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let dir: String = Input::with_theme(theme)
        .with_prompt("Directory to scan")
        .default(default_dir)
        .interact_text()?;
    // Strip surrounding quotes from paths pasted via "Copy as path" or shell echo.
    let cleaned = dir.trim().trim_matches(|c| c == '"' || c == '\'');
    let root = PathBuf::from(cleaned);
    if !root.is_dir() {
        return Err(AppError::NotADirectory(root));
    }

    let mut cfg = ScanConfig::new(root, mode);

    if mode == ScanMode::SimilarImages {
        let labels: Vec<String> = Algorithm::ALL
            .iter()
            .map(|a| format!("{} — {}", a.label(), a.description()))
            .collect();
        let idx = Select::with_theme(theme)
            .with_prompt("Which perceptual algorithm?")
            .items(&labels)
            .default(1)
            .interact()?;
        cfg.algorithm = Algorithm::ALL[idx];

        let threshold: String = Input::with_theme(theme)
            .with_prompt("Similarity threshold (max Hamming distance, 0-64)")
            .default("10".to_string())
            .validate_with(|s: &String| -> std::result::Result<(), String> {
                match s.trim().parse::<u32>() {
                    Ok(v) if v <= 64 => Ok(()),
                    _ => Err("Enter a number between 0 and 64".to_string()),
                }
            })
            .interact_text()?;
        cfg.threshold = threshold.trim().parse().unwrap_or(10);
    }

    if confirm(
        theme,
        "Set advanced filters (size, patterns, hidden, symlinks)?",
        false,
    )? {
        apply_filters(theme, &mut cfg)?;
    }

    Ok(cfg)
}

fn apply_filters(theme: &ColorfulTheme, cfg: &mut ScanConfig) -> Result<()> {
    let min: String = Input::with_theme(theme)
        .with_prompt("Minimum file size in KB (0 = no limit)")
        .default("0".to_string())
        .interact_text()?;
    if let Ok(kb) = min.trim().parse::<u64>() {
        if kb > 0 {
            cfg.min_size = Some(kb * 1024);
        }
    }

    let max: String = Input::with_theme(theme)
        .with_prompt("Maximum file size in KB (0 = no limit)")
        .default("0".to_string())
        .interact_text()?;
    if let Ok(kb) = max.trim().parse::<u64>() {
        if kb > 0 {
            cfg.max_size = Some(kb * 1024);
        }
    }

    let exclude: String = Input::with_theme(theme)
        .with_prompt("Exclude filename globs (comma-separated, blank = none)")
        .allow_empty(true)
        .default(String::new())
        .interact_text()?;
    let patterns: Vec<String> = exclude
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !patterns.is_empty() {
        cfg.exclude = Some(build_globset(&patterns)?);
    }

    cfg.include_hidden = confirm(theme, "Include hidden files/folders?", false)?;
    cfg.follow_symlinks = confirm(theme, "Follow symbolic links?", false)?;
    Ok(())
}

/// Offer export / deletion actions after a scan.
fn post_scan(theme: &ColorfulTheme, results: &ScanResults) -> Result<()> {
    if results.groups.is_empty() {
        return Ok(());
    }
    let can_delete = results.is_exact();

    loop {
        let mut labels: Vec<String> = vec!["Export results (JSON / CSV / text)".to_string()];
        if can_delete {
            labels.push("Delete or move duplicates".to_string());
        }
        labels.push("Continue".to_string());

        let choice = Select::with_theme(theme)
            .with_prompt("Post-scan actions")
            .items(&labels)
            .default(labels.len() - 1)
            .interact()?;

        if choice == 0 {
            export_flow(theme, results)?;
        } else if can_delete && choice == 1 {
            // Files on disk have changed (or the user cancelled) — either way
            // `results` is now stale, so don't loop back into this menu.
            dedupe::run(results)?;
            break;
        } else {
            break;
        }
    }
    Ok(())
}

fn export_flow(theme: &ColorfulTheme, results: &ScanResults) -> Result<()> {
    let format = match Select::with_theme(theme)
        .with_prompt("Export format")
        .items(&["JSON", "CSV", "Plain text"])
        .default(0)
        .interact()?
    {
        0 => Format::Json,
        1 => Format::Csv,
        _ => Format::Text,
    };

    let path: String = Input::with_theme(theme)
        .with_prompt("Output file path")
        .default(format!("duplicates.{}", format.extension()))
        .interact_text()?;
    let path = PathBuf::from(path.trim());

    export::export(results, format, &path)?;
    println!(
        "{} Saved to {}",
        Style::new().green().apply_to("✅"),
        path.display()
    );
    Ok(())
}
