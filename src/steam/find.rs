//! Finds the game folder and the folder of the downloaded mods through Steam. The setup shows
//! what it found, and the `game_find` error names the found game folder. Neither the server nor
//! this search ever uses a found path as a setting (ADR 0007).
//!
//! - The Steam folder: the Windows registry names it (`HKCU\Software\Valve\Steam`, value
//!   `SteamPath`). It is often not the default folder `C:\Program Files (x86)\Steam`, so the
//!   default folder comes second.
//! - The game: the library of app 1874880 in `libraryfolders.vdf` (see `library.rs`).
//! - The mods: `userdata/<user>/config/localconfig.vdf` holds the launch options of each app.
//!   The game downloads mods to `<folder>/addons` of its option `-addonDownloadDir <folder>`.
//!   Without the option, the Workbench folder in Documents holds them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::library::{Libraries, read_vdf};
use crate::host_path::host_path;

/// The Steam app ID of Arma Reforger.
pub const GAME_APP: u32 = 1_874_880;

/// The registry key of Steam for the user.
const STEAM_KEY: &str = r"HKCU\Software\Valve\Steam";

/// The folder where the Steam installer puts Steam when the user does not change it.
const DEFAULT_STEAM: &str = r"C:\Program Files (x86)\Steam";

/// The launch option of the game that names the folder of the downloaded mods.
const DOWNLOAD_OPTION: &str = "-addonDownloadDir";

/// Where the search looks for the Steam folder.
#[derive(Clone, Debug)]
pub enum SteamSearch {
    /// The folder that the Windows registry names, then the default folder.
    Registry,

    /// Only this folder, in Windows form or in the form of this computer.
    #[cfg(test)]
    Folder(String),
}

/// What the search found, in the form of this computer (ADR 0006).
#[derive(Debug)]
pub struct Found {
    pub game: Option<PathBuf>,
    pub mods: Option<Mods>,

    /// Each file or registry value that the search read, and what it gave, in the order read.
    pub checked: Vec<String>,
}

/// The folder that holds `addons` with the downloaded mods.
#[derive(Debug, PartialEq)]
pub struct Mods {
    pub folder: PathBuf,
    pub from: ModsFrom,
}

#[derive(Debug, PartialEq)]
pub enum ModsFrom {
    /// The `-addonDownloadDir` launch option of the game.
    LaunchOptions,

    /// The Workbench folder in Documents, because the launch options name no folder.
    Documents,
}

/// Finds the game folder and the folder of the downloaded mods. `documents` is the Workbench
/// folder in Documents, the folder of the mods when the launch options name none.
pub fn find(search: &SteamSearch, documents: Option<&Path>) -> Found {
    let mut checked = Vec::new();
    let steam = steam_folder(search, &mut checked);
    let game = steam.as_deref().and_then(|steam| game(steam, &mut checked));
    let mods = steam
        .as_deref()
        .and_then(|steam| download_folder(steam, &mut checked))
        .map(|folder| Mods {
            folder,
            from: ModsFrom::LaunchOptions,
        })
        .or_else(|| documents_folder(documents, &mut checked));
    Found {
        game,
        mods,
        checked,
    }
}

/// Finds only the game folder.
pub fn find_game(search: &SteamSearch) -> Option<PathBuf> {
    let mut checked = Vec::new();
    let steam = steam_folder(search, &mut checked)?;
    game(&steam, &mut checked)
}

/// The Steam folder that the Windows registry names, in Windows form, for example `d:/games`.
/// `None` when `reg.exe` does not run or the value is missing.
pub fn steam_from_registry() -> Option<String> {
    let output = Command::new("reg.exe")
        .args(["query", STEAM_KEY, "/v", "SteamPath"])
        .output()
        .ok()?;
    registry_value(&String::from_utf8_lossy(&output.stdout))
}

// region:    --- Support

/// The first Steam folder that has `steamapps/libraryfolders.vdf`.
fn steam_folder(search: &SteamSearch, checked: &mut Vec<String>) -> Option<PathBuf> {
    let candidates: Vec<String> = match search {
        SteamSearch::Registry => {
            let registry = steam_from_registry();
            checked.push(format!(
                "Registry {STEAM_KEY}, value SteamPath: {}",
                registry
                    .as_deref()
                    .unwrap_or("not set, or reg.exe did not run")
            ));
            registry
                .into_iter()
                .chain([DEFAULT_STEAM.to_owned()])
                .collect()
        }
        #[cfg(test)]
        SteamSearch::Folder(folder) => vec![folder.clone()],
    };
    for candidate in candidates {
        let steam = host_path(&candidate);
        let library = steam.join("steamapps/libraryfolders.vdf");
        if library.is_file() {
            return Some(steam);
        }
        checked.push(format!("{}: not found", library.display()));
    }
    None
}

/// The game folder in the libraries of the Steam folder `steam`.
fn game(steam: &Path, checked: &mut Vec<String>) -> Option<PathBuf> {
    let found = Libraries::read(steam).and_then(|libraries| {
        let folder = libraries.app_folder(GAME_APP)?;
        Ok((libraries.file().to_owned(), folder.here))
    });
    match found {
        Ok((file, folder)) => {
            checked.push(format!(
                "{}: the game (app {GAME_APP}) is in {}",
                file.display(),
                folder.display()
            ));
            Some(folder)
        }
        Err(err) => {
            checked.push(err.to_string());
            None
        }
    }
}

/// The folder of `-addonDownloadDir` in the launch options of the game. When Steam has more
/// than one user, the user whose `localconfig.vdf` changed last comes first.
fn download_folder(steam: &Path, checked: &mut Vec<String>) -> Option<PathBuf> {
    let users = steam.join("userdata");
    let Ok(entries) = fs::read_dir(&users) else {
        checked.push(format!("{}: not found", users.display()));
        return None;
    };
    let mut files: Vec<_> = entries
        .filter_map(|user| Some(user.ok()?.path().join("config/localconfig.vdf")))
        .filter(|file| file.is_file())
        .map(|file| (fs::metadata(&file).and_then(|m| m.modified()).ok(), file))
        .collect();
    if files.is_empty() {
        checked.push(format!(
            "{}: no user has config/localconfig.vdf",
            users.display()
        ));
    }
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, file) in files {
        let options = match read_vdf(&file) {
            Ok(top) => top
                .block_at(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
                .and_then(|apps| apps.block(&GAME_APP.to_string()))
                .and_then(|app| app.text("LaunchOptions"))
                .map(str::to_owned),
            Err(err) => {
                checked.push(err.to_string());
                continue;
            }
        };
        let Some(raw) = options.as_deref().and_then(option_value) else {
            checked.push(format!(
                "{}: the launch options of the game have no {DOWNLOAD_OPTION}",
                file.display()
            ));
            continue;
        };
        let folder = host_path(&raw);
        if folder.is_dir() {
            checked.push(format!("{}: {DOWNLOAD_OPTION} \"{raw}\"", file.display()));
            return Some(folder);
        }
        checked.push(format!(
            "{}: {DOWNLOAD_OPTION} \"{raw}\", but {} does not exist",
            file.display(),
            folder.display()
        ));
    }
    None
}

/// The Workbench folder in Documents, when it exists.
fn documents_folder(documents: Option<&Path>, checked: &mut Vec<String>) -> Option<Mods> {
    let Some(folder) = documents.filter(|folder| folder.is_dir()) else {
        checked.push("The Workbench folder in Documents: not found".to_owned());
        return None;
    };
    checked.push(format!(
        "{}: the Workbench folder in Documents",
        folder.display()
    ));
    Some(Mods {
        folder: folder.to_owned(),
        from: ModsFrom::Documents,
    })
}

/// The value after `-addonDownloadDir` in a line of launch options, without its quotes.
fn option_value(options: &str) -> Option<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for c in options.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            c => word.push(c),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    let mut words = words.into_iter();
    words.find(|word| word.eq_ignore_ascii_case(DOWNLOAD_OPTION))?;
    words.next().filter(|value| !value.starts_with('-'))
}

/// The value of a `REG_SZ` line in the output of `reg.exe query`.
fn registry_value(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| Some(line.split_once("REG_SZ")?.1.trim().to_owned()))
        .filter(|path| !path.is_empty())
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;

// endregion: --- Tests
