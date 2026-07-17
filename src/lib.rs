//! DupFile-Analyzer — interactive duplicate file & similar-image finder.

pub mod app;
pub mod config;
pub mod dedupe;
pub mod error;
pub mod export;
pub mod hashing;
pub mod report;
pub mod scanner;

pub use error::{AppError, Result};

/// Run the interactive application, returning a process exit code.
pub fn run() -> i32 {
    match app::run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!(
                "{} {}",
                console::Style::new().red().bold().apply_to("❌"),
                e
            );
            1
        }
    }
}
