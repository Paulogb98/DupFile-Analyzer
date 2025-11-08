mod utils;

use crate::utils::{process_directory, report_duplicates, FileHashError};
use clap::{ArgAction, Parser};
use console::Style;
use std::path::PathBuf;

/// Programa para encontrar arquivos duplicados em um diretório com base em hashes SHA-256.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Caminho do diretório a ser processado
    #[arg(value_name = "DIRETÓRIO", value_parser = validate_dir)]
    dir_path: PathBuf,

    /// Modo silencioso (não exibe mensagens informativas)
    #[arg(short, long, action = ArgAction::SetTrue)]
    quiet: bool,
}

fn validate_dir(path_str: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path_str);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!(
            "O caminho '{}' não é um diretório válido.",
            path_str
        ))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.quiet {
        let info_style = Style::new().cyan().bold();
        println!(
            "\n{} Processando diretório: {}",
            info_style.apply_to("ℹ️ "),
            args.dir_path.display()
        );
    }

    match process_directory(&args.dir_path) {
        Ok(file_hash_map) => {
            report_duplicates(file_hash_map, args.quiet)?;
        }

        Err(FileHashError::EmptyDirectory(path)) => {
            let warn_style = Style::new().yellow().bold();
            println!(
                "{} Nenhum arquivo encontrado no diretório: {}",
                warn_style.apply_to("⚠️"),
                path.display()
            );
        }

        Err(err) => {
            let error_style = Style::new().red().bold();
            eprintln!("{} Falha ao processar: {}", error_style.apply_to("❌"), err);
        }
    }

    Ok(())
}
