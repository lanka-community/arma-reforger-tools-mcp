//! Lists the Knowledge base page files and the files of the sample mods, so that `include_str!`
//! and `include_bytes!` put each of them into the server program.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(
        std::env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR is not set"))?,
    );

    let pages_dir = Path::new("src/wiki/pages");
    println!("cargo::rerun-if-changed={}", pages_dir.display());
    let mut pages = Vec::new();
    collect_files(pages_dir, &mut pages)?;
    pages.retain(|path| path.extension().is_some_and(|ext| ext == "md"));
    fs::write(
        out_dir.join("wiki_pages.rs"),
        file_list(pages_dir, pages, "include_str")?,
    )?;

    // Each folder in `samples` is one sample mod. The files next to these folders are the
    // license and the README of the copy, not files of a mod.
    let samples_dir = Path::new("src/game-files/samples");
    println!("cargo::rerun-if-changed={}", samples_dir.display());
    let mut samples = Vec::new();
    for entry in fs::read_dir(samples_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, &mut samples)?;
        }
    }
    fs::write(
        out_dir.join("sample_files.rs"),
        file_list(samples_dir, samples, "include_bytes")?,
    )
}

/// The Rust text of a list of (path under `dir`, `include(file)`), sorted by path.
fn file_list(dir: &Path, mut files: Vec<PathBuf>, include: &str) -> io::Result<String> {
    files.sort();
    let mut list = String::from("&[\n");
    for file in files {
        let relative = file
            .strip_prefix(dir)
            .map_err(io::Error::other)?
            .to_string_lossy()
            .replace('\\', "/");
        let absolute = fs::canonicalize(&file)?;
        let _ = writeln!(
            list,
            "    ({relative:?}, {include}!({:?})),",
            absolute.to_string_lossy()
        );
    }
    list.push(']');
    Ok(list)
}

/// Adds every file in `dir` and its subfolders to `files`.
fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}
