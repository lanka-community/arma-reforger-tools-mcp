//! `cargo xtask import --samples <clone>`: copies the text files of Bohemia's sample mods from a
//! local clone of `BohemiaInteractive/Arma-Reforger-Samples` into `src/game-files/samples/`.
//!
//! The clone is outside data. This module only reads its files: it runs no program in it, and it
//! follows no symlink.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::knowledge::{remove_empty_dirs, write_if_changed};
use crate::{Error, Result};

/// The GitHub repo of the samples.
const SOURCE_REPO: &str = "https://github.com/BohemiaInteractive/Arma-Reforger-Samples";

/// Each folder of a sample mod starts with this.
const MOD_PREFIX: &str = "SampleMod_";

/// The extensions of the text files that Claude can read: scripts, Prefabs, configs, layouts,
/// project files, metadata, worlds, layers, and string tables.
const TEXT_EXTENSIONS: &[&str] = &[
    "c", "et", "ct", "conf", "layout", "gproj", "meta", "ent", "layer", "st",
];

/// The files that hold the license and the copyright notice. The import copies them too.
const NOTICE_FILES: &[&str] = &["README.md", "license.txt"];

/// The largest text file of the samples has 85 KB. A larger file is a sign of a wrong folder.
const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// The samples have folders 10 levels deep. A deeper folder is a sign of a wrong folder.
const MAX_DEPTH: usize = 32;

/// Copies the text files of each sample mod in `clone` to `<repo>/src/game-files/samples/`, with
/// the license of the samples and a README that names the source commit. A file that is gone
/// from the samples is deleted, and a file that did not change is not written.
pub fn import(repo: &Path, clone: &Path) -> Result<()> {
    let license = clone.join("LICENSE");
    if !license.is_file() {
        return Err(Error::NotASamplesClone {
            path: clone.to_owned(),
        });
    }
    let commit = head_commit(clone)?;
    let mut wanted: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    wanted.insert("LICENSE".to_owned(), read(&license)?);
    let mods = sample_mods(clone)?;
    if mods.is_empty() {
        return Err(Error::NotASamplesClone {
            path: clone.to_owned(),
        });
    }
    for name in &mods {
        let mut files = Vec::new();
        collect_files(&clone.join(name), 0, &mut files)?;
        for file in files.into_iter().filter(|file| is_copied(file)) {
            let relative = file
                .strip_prefix(clone)
                .map_or_default(|path| path.to_string_lossy().replace('\\', "/"));
            wanted.insert(relative, read(&file)?);
        }
    }
    let count = wanted.len() - 1;
    wanted.insert("README.md".to_owned(), readme(&commit).into_bytes());

    let dir = repo.join("src/game-files/samples");
    let mut deleted = 0;
    let mut old_files = Vec::new();
    collect_files(&dir, 0, &mut old_files)?;
    for old in old_files {
        let relative = old
            .strip_prefix(&dir)
            .map_or_default(|path| path.to_string_lossy().replace('\\', "/"));
        if !wanted.contains_key(&relative) {
            fs::remove_file(&old).map_err(|source| Error::Write { path: old, source })?;
            deleted += 1;
        }
    }
    let mut written = 0;
    for (path, bytes) in &wanted {
        if write_if_changed(&dir.join(path), bytes)? {
            written += 1;
        }
    }
    remove_empty_dirs(&dir)?;
    println!(
        "Samples: {count} files of {} sample mods from commit {commit}. {written} written, \
         {deleted} deleted, {} unchanged.",
        mods.len(),
        wanted.len() - written
    );
    Ok(())
}

// region:    --- Support

/// The folder names of the sample mods in `clone`, sorted.
fn sample_mods(clone: &Path) -> Result<Vec<String>> {
    let entries = fs::read_dir(clone).map_err(|source| Error::Read {
        path: clone.to_owned(),
        source,
    })?;
    let mut mods: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().ok()?.is_dir();
            (is_dir && name.starts_with(MOD_PREFIX)).then_some(name)
        })
        .collect();
    mods.sort();
    Ok(mods)
}

/// Whether the import copies `file`: a text file by its extension, or a notice file by its name.
/// Capital letters do not matter.
fn is_copied(file: &Path) -> bool {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let is_notice = NOTICE_FILES
        .iter()
        .any(|notice| name.eq_ignore_ascii_case(notice));
    let is_text = file
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| TEXT_EXTENSIONS.iter().any(|t| ext.eq_ignore_ascii_case(t)));
    is_notice || is_text
}

/// Adds every file in `dir` and its subfolders to `files`. A missing folder adds nothing.
/// `file_type` does not follow a symlink, and a symlink is not added, so the walk never leaves
/// `dir`.
fn collect_files(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Error::SamplesTooDeep {
            path: dir.to_owned(),
        });
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_owned(),
            source,
        })?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        if kind.is_dir() {
            collect_files(&path, depth + 1, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Reads one file of the clone. A file above `MAX_FILE_BYTES` is an error.
fn read(path: &Path) -> Result<Vec<u8>> {
    let len = fs::metadata(path)
        .map_err(|source| Error::Read {
            path: path.to_owned(),
            source,
        })?
        .len();
    if len > MAX_FILE_BYTES {
        return Err(Error::SampleFileTooLarge {
            path: path.to_owned(),
            len,
        });
    }
    fs::read(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })
}

/// The commit that the clone has checked out. It reads the files in `.git` and runs no `git`
/// program, because the clone is outside data.
fn head_commit(clone: &Path) -> Result<String> {
    let git = clone.join(".git");
    let head = read_text(&git.join("HEAD"))?;
    let head = head.trim();
    let commit = match head.strip_prefix("ref: ") {
        None => head.to_owned(),
        Some(name) => {
            // The name comes from the clone, so it must stay inside `.git/refs`.
            if !name.starts_with("refs/") || name.split('/').any(|part| part == "..") {
                return Err(Error::NoSamplesCommit { path: git });
            }
            match fs::read_to_string(git.join(name)) {
                Ok(text) => text.trim().to_owned(),
                Err(_) => packed_ref(&read_text(&git.join("packed-refs"))?, name)
                    .ok_or_else(|| Error::NoSamplesCommit { path: git.clone() })?,
            }
        }
    };
    if commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(commit)
    } else {
        Err(Error::NoSamplesCommit { path: git })
    }
}

/// The commit of the ref `name` in the text of a `packed-refs` file.
fn packed_ref(packed: &str, name: &str) -> Option<String> {
    packed.lines().find_map(|line| {
        let (commit, line_name) = line.split_once(' ')?;
        (line_name == name).then(|| commit.to_owned())
    })
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })
}

/// The README of the copied folder. It names the source, the commit, and the license, as the
/// Arma Public License asks in its Section 3(a)(1).
fn readme(commit: &str) -> String {
    let extensions: Vec<String> = TEXT_EXTENSIONS
        .iter()
        .map(|ext| format!("`.{ext}`"))
        .collect();
    format!(
        "# Bohemia's sample mods\n\
         \n\
         The text files of the sample mods of Bohemia Interactive. The server program holds \
         them. `game_find` and `game_read` show each sample mod as a source, for example \
         `(sample SampleMod_NewWeapon)`, and `game_read` opens a file of it with \
         `source=SampleMod_NewWeapon`.\n\
         \n\
         - Source: {SOURCE_REPO}\n\
         - Commit: `{commit}`\n\
         - Copyright: © 2022 Bohemia Interactive a.s. All rights reserved.\n\
         - License: Arma Public License (APL), in `LICENSE`. The full text is at \
         https://www.bohemia.net/community/licenses/arma-public-license. \
         `SampleMod_Main/license.txt` is the license file that the sample `SampleMod_Main` comes \
         with.\n\
         \n\
         The MIT license of this repo does not cover this folder. Use these files only for \
         non-commercial work on Arma games, as the APL says. Bohemia Interactive does not support \
         or endorse this plugin.\n\
         \n\
         `cargo xtask import --samples <clone>` copied only the text files ({}) and the \
         `README.md` and `license.txt` files. It did not change them. The models, textures, \
         sounds, animations, and other binary files are not here. Get them from the source \
         repo.\n",
        extensions.join(", ")
    )
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_only_text_and_notice_files() {
        // -- Exec & Check
        for (file, copied) in [
            ("SampleMod_NewWeapon/Prefabs/Weapons/Rifle.et", true),
            ("SampleMod_NewWeapon/SampleMod_NewWeapon.gproj", true),
            ("SampleMod_NewWeapon/Assets/Rifle.xob.meta", true),
            ("SampleMod_Main/license.txt", true),
            ("SampleMod_NewWeapon/README.md", true),
            ("SampleMod_NewWeapon/Assets/Rifle.xob", false),
            ("SampleMod_NewWeapon/Assets/Rifle.edds", false),
            ("SampleMod_NewWeapon/thumbnail.png", false),
            ("SampleMod_NewWeapon/Anims/bolt.txa", false),
        ] {
            assert_eq!(is_copied(Path::new(file)), copied, "{file}");
        }
    }

    #[test]
    fn copies_the_text_files_of_each_sample_mod_with_the_license_and_the_commit() {
        // -- Setup & Fixtures
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/.out/copies_the_text_files_of_each_sample_mod");
        assert!(root.to_string_lossy().contains("tests/.out"));
        let _ = fs::remove_dir_all(&root);
        let clone = root.join("clone");
        let repo = root.join("repo");
        let commit = "83f12390d3dd61ed834d379f621520ed9f1b891d";
        for (path, text) in [
            ("LICENSE", "Arma Public License (APL)\n"),
            (".git/HEAD", "ref: refs/heads/main\n"),
            (
                ".git/refs/heads/main",
                "83f12390d3dd61ed834d379f621520ed9f1b891d\n",
            ),
            ("README.md", "The README of the whole repo.\n"),
            (
                "SampleMod_NewWeapon/README.md",
                "© 2022 Bohemia Interactive a.s.\n",
            ),
            (
                "SampleMod_NewWeapon/Prefabs/Rifle.et",
                "GenericEntity {\n}\n",
            ),
            ("SampleMod_NewWeapon/Assets/Rifle.xob", "binary model"),
            ("Tools/Script.c", "class NotASampleMod {}\n"),
        ] {
            let file = clone.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
        let samples = repo.join("src/game-files/samples");
        let gone = samples.join("SampleMod_Gone/Gone.c");
        fs::create_dir_all(gone.parent().unwrap()).unwrap();
        fs::write(&gone, "class Gone {}\n").unwrap();

        // -- Exec
        import(&repo, &clone).unwrap();

        // -- Check
        let mut copied = Vec::new();
        collect_files(&samples, 0, &mut copied).unwrap();
        let mut copied: Vec<String> = copied
            .iter()
            .map(|file| {
                let relative = file.strip_prefix(&samples).unwrap();
                relative.to_string_lossy().replace('\\', "/")
            })
            .collect();
        copied.sort();
        assert_eq!(
            copied,
            [
                "LICENSE",
                "README.md",
                "SampleMod_NewWeapon/Prefabs/Rifle.et",
                "SampleMod_NewWeapon/README.md",
            ]
        );
        assert!(!samples.join("SampleMod_Gone").exists());
        assert_eq!(
            fs::read_to_string(samples.join("LICENSE")).unwrap(),
            "Arma Public License (APL)\n"
        );
        assert_eq!(
            fs::read_to_string(samples.join("SampleMod_NewWeapon/Prefabs/Rifle.et")).unwrap(),
            "GenericEntity {\n}\n"
        );
        let readme = fs::read_to_string(samples.join("README.md")).unwrap();
        assert!(
            readme.contains(&format!("- Commit: `{commit}`")),
            "{readme}"
        );
    }

    #[test]
    fn finds_the_commit_of_a_ref_in_packed_refs() {
        // -- Setup & Fixtures
        let packed = "# pack-refs with: peeled fully-peeled sorted\n\
                      83f12390d3dd61ed834d379f621520ed9f1b891d refs/heads/main\n";

        // -- Exec & Check
        assert_eq!(
            packed_ref(packed, "refs/heads/main").as_deref(),
            Some("83f12390d3dd61ed834d379f621520ed9f1b891d")
        );
        assert_eq!(packed_ref(packed, "refs/heads/other"), None);
    }

    #[test]
    fn refuses_a_head_ref_outside_the_refs_folder() {
        // -- Setup & Fixtures
        let clone = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/.out/refuses_a_head_ref_outside_the_refs_folder");
        assert!(clone.to_string_lossy().contains("tests/.out"));
        let _ = fs::remove_dir_all(&clone);
        fs::create_dir_all(clone.join(".git")).unwrap();
        fs::write(clone.join(".git/HEAD"), "ref: refs/../../secret\n").unwrap();

        // -- Exec
        let result = head_commit(&clone);

        // -- Check
        assert!(matches!(result, Err(Error::NoSamplesCommit { .. })));
    }
}

// endregion: --- Tests
