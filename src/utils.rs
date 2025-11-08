use console::{style, Style};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use thiserror::Error;
use walkdir::WalkDir;

/// Tamanho do buffer para leitura dos arquivos (64KB é um valor seguro e eficiente).
const BUFFER_SIZE: usize = 65536; // 64KB

/// Erros customizados para operações de hash e leitura de diretórios.
#[derive(Error, Debug)]
pub enum FileHashError {
    #[error("Erro de IO ao acessar '{0}': {1}")]
    IoError(PathBuf, #[source] io::Error),

    #[error("Nenhum arquivo encontrado no diretório '{0}'")]
    EmptyDirectory(PathBuf),
}

/// Calcula o hash SHA256 de um arquivo usando um buffer eficiente.
pub fn calculate_sha256(file_path: &Path) -> Result<String, FileHashError> {
    let file =
        File::open(file_path).map_err(|e| FileHashError::IoError(file_path.to_path_buf(), e))?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| FileHashError::IoError(file_path.to_path_buf(), e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Processa um diretório e retorna um mapa de hashes para arquivos.
pub fn process_directory(path: &Path) -> Result<HashMap<String, Vec<PathBuf>>, FileHashError> {
    let entries: Vec<PathBuf> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.into_path())
        .collect();

    if entries.is_empty() {
        return Err(FileHashError::EmptyDirectory(path.to_path_buf()));
    }

    println!(
        "{} {} arquivos encontrados. Processando...\n",
        style("✔️ ").green(),
        entries.len()
    );

    let pb = Arc::new(Mutex::new(ProgressBar::new(entries.len() as u64)));
    pb.lock().unwrap().set_style(
        ProgressStyle::with_template(
            "  {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    let progress_counter = Arc::new(AtomicU64::new(0));

    // Thread que atualiza a progress bar
    let pb_counter = Arc::clone(&progress_counter);
    let pb_lock = Arc::clone(&pb);
    let pb_thread = thread::spawn(move || {
        loop {
            let count = pb_counter.load(Ordering::Relaxed);
            pb_lock.lock().unwrap().set_position(count);

            if count >= pb_lock.lock().unwrap().length().unwrap() {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // Faz o cálculo dos hashes paralelamente
    let results: Vec<Result<(String, PathBuf), FileHashError>> = entries
        .par_iter()
        .map(|file_path| {
            let res = calculate_sha256(file_path).map(|hash| (hash, file_path.clone()));

            progress_counter.fetch_add(1, Ordering::Relaxed);

            res
        })
        .collect();

    pb_thread.join().unwrap();

    pb.lock().unwrap().finish();
    println!("\n");

    let mut file_hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

    results.into_iter().for_each(|res| match res {
        Ok((hash, file_path)) => {
            file_hash_map
                .entry(hash)
                .or_insert_with(Vec::new)
                .push(file_path);
        }
        Err(e) => {
            eprintln!("{} Falha ao processar arquivo: {}", style("⚠️").yellow(), e);
        }
    });

    Ok(file_hash_map)
}

/// Exibe um relatório de arquivos duplicados com base no mapa de hashes.
/// `quiet`: se true, suprime mensagens informativas quando não houver duplicatas.
pub fn report_duplicates(
    file_hash_map: HashMap<String, Vec<PathBuf>>,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let hash_style = Style::new().yellow().bold();
    let file_style = Style::new().green();

    let duplicates: Vec<_> = file_hash_map
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();

    if !duplicates.is_empty() {
        println!(
            "{} Foram encontradas {} duplicatas\n",
            Style::new().cyan().bold().apply_to("ℹ️ "),
            duplicates.len()
        );

        for (hash, files) in duplicates {
            println!("{} {}", hash_style.apply_to("Hash duplicado:"), hash);
            for file in files {
                println!("  - {}", file_style.apply_to(file.display()));
            }
            println!();
        }
    } else if !quiet {
        let success_style = Style::new().green().bold();
        println!(
            "{} Nenhum arquivo duplicado encontrado.",
            success_style.apply_to("✅")
        );
    }

    Ok(())
}
