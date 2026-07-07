//! Migrate legacy `<!-- -->` annotations in a vault to `<!--- --->` form.
//!
//! Usage: migrate <vault-path> [--dry-run] [--ext md]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use annotation_core::migrate::migrate_content;

fn main() {
    let mut vault: Option<PathBuf> = None;
    let mut dry_run = false;
    let mut ext = String::from("md");

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--ext" => match args.next() {
                Some(e) => ext = e,
                None => usage_exit("--ext requires a value"),
            },
            _ if vault.is_none() => vault = Some(PathBuf::from(arg)),
            _ => usage_exit(&format!("unexpected argument: {arg}")),
        }
    }

    let Some(vault) = vault else {
        usage_exit("missing vault path");
    };

    let mut files = Vec::new();
    collect_files(&vault, &ext, &mut files);
    files.sort();

    let mut changed_files = 0usize;
    let mut total_conversions = 0usize;

    for path in files {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping {}: {e}", path.display());
                continue;
            }
        };

        let result = migrate_content(&content);
        if result.conversions == 0 {
            continue;
        }

        if !dry_run {
            if let Err(e) = fs::write(&path, &result.output) {
                eprintln!("failed to write {}: {e}", path.display());
                exit(1);
            }
        }

        println!("{}: {} conversions", path.display(), result.conversions);
        changed_files += 1;
        total_conversions += result.conversions;
    }

    let suffix = if dry_run { " (dry run)" } else { "" };
    println!("Total: {changed_files} files changed, {total_conversions} conversions{suffix}");
}

/// Recursively collect files with the given extension, skipping hidden
/// directories (e.g. `.obsidian`).
fn collect_files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        eprintln!("cannot read directory {}", dir.display());
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if path.is_dir() {
            if !name.starts_with('.') {
                collect_files(&path, ext, out);
            }
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("error: {msg}");
    eprintln!("usage: migrate <vault-path> [--dry-run] [--ext md]");
    exit(1);
}
