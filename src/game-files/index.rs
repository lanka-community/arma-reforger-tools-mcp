//! The list of every file in the .pak archives of the game and of the downloaded Workshop mods,
//! and of every file of Bohemia's sample mods in the server program.

use std::collections::HashSet;
use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use tokio_util::sync::CancellationToken;

use super::pak::{FileData, PakFile, list_files, read_file};
use super::settings::{GAME_SETTING, PROFILE_SETTING, Settings};
use super::{Error, Result};
use crate::host_path::host_path;
use crate::mod_folder::ModFolder;
use crate::workbench_documents::DOCUMENTS_SETTING;

/// Every Game file, with its sources and notes about what was not searched.
pub(super) struct Index {
    pub(super) files: Vec<GameFile>,

    /// `Source::Game` first when the game folder is set, then each mod, then each sample mod.
    pub(super) sources: Vec<Source>,

    /// Each line says what the search did not cover and how to fix it.
    pub(super) notes: Vec<String>,

    /// The mods that the index was built from. When the mods on disk differ, the index is old.
    pub(super) mods: ModsOnDisk,
}

/// The mod folders in both `addons` folders, each with its .pak files. Two lists are equal when
/// no folder and no .pak file came, went, or changed its size or its modified time.
#[derive(PartialEq, Eq)]
pub(super) struct ModsOnDisk(Vec<ModOnDisk>);

#[derive(PartialEq, Eq)]
struct ModOnDisk {
    folder: PathBuf,
    paks: Vec<PakStamp>,
}

/// A .pak file as the file system gives it, with no read of its content.
#[derive(PartialEq, Eq)]
struct PakStamp {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

impl ModsOnDisk {
    /// Lists the mod folders and their .pak files. It reads no .pak file.
    pub(super) fn list(settings: &Settings) -> Self {
        Self::of(mod_folders(settings).folders)
    }

    fn of(folders: Vec<PathBuf>) -> Self {
        Self(
            folders
                .into_iter()
                .map(|folder| {
                    let mut paks = Vec::new();
                    collect_paks(&folder, 1, &mut paks);
                    let paks = paks
                        .into_iter()
                        .map(|path| {
                            let meta = fs::metadata(&path).ok();
                            PakStamp {
                                len: meta.as_ref().map_or(0, fs::Metadata::len),
                                modified: meta.and_then(|meta| meta.modified().ok()),
                                path,
                            }
                        })
                        .collect();
                    ModOnDisk { folder, paks }
                })
                .collect(),
        )
    }
}

/// Where a Game file comes from.
pub(super) enum Source {
    /// The installed game.
    Game,
    /// A downloaded Workshop mod, with its name, for example `tvtLobby`.
    Mod(String),
    /// A sample mod of Bohemia, with its folder name, for example `SampleMod_NewWeapon`.
    Sample(String),
}

impl Source {
    /// Whether the user named this source with `wanted`: `game`, `tvtLobby`, `mod tvtLobby`,
    /// `SampleMod_NewWeapon`, or `sample SampleMod_NewWeapon`. Capital letters do not matter.
    pub(super) fn is(&self, wanted: &str) -> bool {
        let wanted = wanted.trim();
        match self {
            Self::Game => wanted.eq_ignore_ascii_case("game"),
            Self::Mod(name) | Self::Sample(name) => {
                wanted.eq_ignore_ascii_case(name) || wanted.eq_ignore_ascii_case(&self.to_string())
            }
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game => f.write_str("game"),
            Self::Mod(name) => write!(f, "mod {name}"),
            Self::Sample(name) => write!(f, "sample {name}"),
        }
    }
}

/// One Game file.
pub(super) struct GameFile {
    pub(super) path: String,

    /// The position of its source in `Index::sources`.
    pub(super) source: usize,

    pub(super) holder: Holder,
}

/// What holds the content of a Game file.
pub(super) enum Holder {
    /// A .pak file on disk, at a position in it.
    Pak { pak: Arc<Path>, data: FileData },
    /// The server program: `build.rs` put the file into it.
    Program(&'static [u8]),
}

impl GameFile {
    /// Reads its full content, from its .pak file or from the server program.
    pub(super) fn read(&self) -> Result<Vec<u8>> {
        match &self.holder {
            Holder::Pak { pak, data } => read_file(&mut File::open(pak)?, *data),
            Holder::Program(bytes) => Ok(bytes.to_vec()),
        }
    }

    /// The .pak file that holds it, or `None` for a file in the server program.
    pub(super) fn pak(&self) -> Option<&Path> {
        match &self.holder {
            Holder::Pak { pak, .. } => Some(pak),
            Holder::Program(_) => None,
        }
    }
}

/// Reads the file lists of all .pak archives of the game and of the mods, and adds the files of
/// the sample mods in `settings`. When the game folder is not set, the index has no files of the
/// game, and a note names the setting. It fails when the game folder is set but has no .pak
/// files. A damaged .pak file or a missing mods folder becomes a note. It checks `cancel` before
/// each .pak file.
pub(super) fn build(settings: &Settings, cancel: &CancellationToken) -> Result<Index> {
    let mut index = Index {
        files: Vec::new(),
        sources: Vec::new(),
        notes: Vec::new(),
        mods: ModsOnDisk(Vec::new()),
    };
    match settings.game.as_deref() {
        None => index.notes.push(format!(
            "{GAME_SETTING} is not set, so the search covered no files of the game. Set it to the \
             game folder, the folder that holds ArmaReforger.exe, for example \
             \"D:\\SteamLibrary\\steamapps\\common\\Arma Reforger\" or \
             \"/mnt/d/SteamLibrary/steamapps/common/Arma Reforger\". Then restart Claude Code."
        )),
        Some(raw_game) => {
            let game_folder = host_path(raw_game);
            let mut game_paks = Vec::new();
            collect_paks(&game_folder.join("addons"), 2, &mut game_paks);
            if game_paks.is_empty() {
                return Err(Error::NoGamePaks {
                    value: raw_game.to_owned(),
                    folder: game_folder,
                });
            }
            index.sources.push(Source::Game);
            index.add(0, &game_paks, cancel)?;
        }
    }
    let ModFolders { folders, notes } = mod_folders(settings);
    index.notes.extend(notes);
    let mods = ModsOnDisk::of(folders);
    for ModOnDisk { folder, paks } in &mods.0 {
        if paks.is_empty() {
            continue;
        }
        let paks: Vec<PathBuf> = paks.iter().map(|pak| pak.path.clone()).collect();
        index.sources.push(Source::Mod(mod_name(folder)));
        index.add(index.sources.len() - 1, &paks, cancel)?;
    }
    index.mods = mods;
    index.add_samples(settings.samples);
    Ok(index)
}

/// The mod folders in both `addons` folders, with notes about a folder that was not searched.
struct ModFolders {
    /// Sorted by folder name. A mod ID that is in both `addons` folders is here once, from the
    /// Documents folder.
    folders: Vec<PathBuf>,
    notes: Vec<String>,
}

/// Lists the mod folders in `<Documents folder>/addons` and in `<profile>/addons`. A folder that
/// is missing becomes a note.
fn mod_folders(settings: &Settings) -> ModFolders {
    let mut notes = Vec::new();
    let mut folders = Vec::new();
    if let Some(documents) = &settings.documents {
        folders.extend(sub_folders(&documents.join("addons")).unwrap_or_default());
    }
    let has_documents = settings.documents.is_some();
    match settings.profile.as_deref() {
        None if !has_documents => notes.push(format!(
            "{DOCUMENTS_SETTING} is not set and the Tool found no Workbench folder in Documents, \
             so the search covered no downloaded Workshop mods. To also search the Workshop mods \
             that Workbench downloaded, set {DOCUMENTS_SETTING} to the Workbench folder in \
             Documents, for example \"C:\\Users\\<you>\\Documents\\My Games\\ArmaReforgerWorkbench\". \
             To also search another folder of Workshop mods, for example \"D:\\DownloadedMods\", \
             set {PROFILE_SETTING} to it."
        )),
        None => {}
        Some(raw_profile) => {
            let mods_folder = host_path(raw_profile).join("addons");
            if let Some(profile_folders) = sub_folders(&mods_folder) {
                let in_documents: HashSet<String> = folders.iter().map(|f| mod_key(f)).collect();
                folders.extend(
                    profile_folders
                        .into_iter()
                        .filter(|folder| !in_documents.contains(&mod_key(folder))),
                );
            } else {
                let covered = if has_documents {
                    "the search did not cover its mods"
                } else {
                    "the search covered no downloaded Workshop mods"
                };
                notes.push(format!(
                    "{PROFILE_SETTING} is \"{raw_profile}\", but {} is not a folder, so \
                     {covered}. Set it to a folder of Workshop mods that holds addons, for \
                     example \"D:\\DownloadedMods\".",
                    mods_folder.display()
                ));
            }
        }
    }
    folders.sort_by_key(|folder| folder.file_name().map(ToOwned::to_owned));
    ModFolders { folders, notes }
}

impl Index {
    /// Where `file` comes from: `game`, `mod tvtLobby`, or `sample SampleMod_NewWeapon`.
    pub(super) fn source_of(&self, file: &GameFile) -> String {
        self.sources
            .get(file.source)
            .map_or_else(|| Source::Game.to_string(), ToString::to_string)
    }

    /// Whether `file` comes from the source that the user named `wanted` (see `Source::is`).
    pub(super) fn is_source(&self, file: &GameFile, wanted: &str) -> bool {
        self.sources
            .get(file.source)
            .is_some_and(|source| source.is(wanted))
    }

    /// What the search covered: "the game", "the game and 2 mods (tvtLobby, Other)", "the game,
    /// 1 mod (tvtLobby), and 13 sample mods", or "13 sample mods" when the game folder is not set.
    pub(super) fn searched(&self) -> String {
        let mods: Vec<&str> = self
            .sources
            .iter()
            .filter_map(|source| match source {
                Source::Mod(name) => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let samples = self
            .sources
            .iter()
            .filter(|source| matches!(source, Source::Sample(_)))
            .count();
        let mut parts = Vec::new();
        if self
            .sources
            .iter()
            .any(|source| matches!(source, Source::Game))
        {
            parts.push("the game".to_owned());
        }
        match mods.len() {
            0 => {}
            1 => parts.push(format!("1 mod ({})", mods.join(", "))),
            count => parts.push(format!("{count} mods ({})", mods.join(", "))),
        }
        match samples {
            0 => {}
            1 => parts.push("1 sample mod".to_owned()),
            count => parts.push(format!("{count} sample mods")),
        }
        match parts.as_slice() {
            [last] => last.clone(),
            [first, last] => format!("{first} and {last}"),
            [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
            [] => "no source".to_owned(),
        }
    }

    /// Adds the files of the sample mods. Each path is `<sample mod folder>/<path in the mod>`,
    /// and the folder name becomes the source. A path without a folder is skipped.
    fn add_samples(&mut self, samples: &[(&str, &'static [u8])]) {
        for &(path, bytes) in samples {
            let Some((name, path)) = path.split_once('/') else {
                continue;
            };
            let known = self
                .sources
                .iter()
                .position(|source| matches!(source, Source::Sample(known) if known == name));
            let source = known.unwrap_or_else(|| {
                self.sources.push(Source::Sample(name.to_owned()));
                self.sources.len() - 1
            });
            self.files.push(GameFile {
                path: path.to_owned(),
                source,
                holder: Holder::Program(bytes),
            });
        }
    }

    /// Adds the files of `paks` from the source at `source`.
    fn add(&mut self, source: usize, paks: &[PathBuf], cancel: &CancellationToken) -> Result<()> {
        for pak in paks {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            match read_pak(pak) {
                Ok(files) => {
                    let pak: Arc<Path> = Arc::from(pak.as_path());
                    self.files
                        .extend(files.into_iter().map(|PakFile { path, data }| GameFile {
                            path,
                            source,
                            holder: Holder::Pak {
                                pak: Arc::clone(&pak),
                                data,
                            },
                        }));
                }
                Err(error) => self.notes.push(format!(
                    "Skipped {}: {error}. The file may be damaged: verify the game files in Steam, \
                     or download the mod again.",
                    pak.display()
                )),
            }
        }
        Ok(())
    }
}

// region:    --- Support

fn read_pak(pak: &Path) -> Result<Vec<PakFile>> {
    let mut file = File::open(pak)?;
    let len = file.metadata()?.len();
    list_files(&mut file, len)
}

/// Adds the .pak files in `folder` to `paks`, and those in its subfolders down to `depth` levels.
/// The game keeps them in `addons/<name>/`, a mod in `addons/<mod>/`.
fn collect_paks(folder: &Path, depth: usize, paks: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            if depth > 1 {
                collect_paks(&entry, depth - 1, paks);
            }
        } else if entry
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pak"))
        {
            paks.push(entry);
        }
    }
}

/// The folders in `folder`, or `None` when it is not a folder.
fn sub_folders(folder: &Path) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(folder).ok()?;
    Some(
        entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|folder| folder.is_dir())
            .collect(),
    )
}

/// What makes two mod folders the same mod: the ID of `<Name>_<ID>` in capitals, or else the
/// whole folder name.
fn mod_key(folder: &Path) -> String {
    let name = folder.file_name().unwrap_or_default().to_string_lossy();
    match ModFolder::parse(&name) {
        Some(parts) => parts.id.to_ascii_uppercase(),
        None => name.into_owned(),
    }
}

/// The name of a mod from its folder `<Name>_<ID>`, or else the whole folder name.
fn mod_name(folder: &Path) -> String {
    let folder = folder.file_name().unwrap_or_default().to_string_lossy();
    match ModFolder::parse(&folder) {
        Some(parts) => parts.name.to_owned(),
        None => folder.into_owned(),
    }
}

// endregion: --- Support
