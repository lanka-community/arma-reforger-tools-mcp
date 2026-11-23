//! A Steam folder for tests, made from the sample files in `samples/`. They are copies of the
//! files of a real Steam install from 2026, with the library `D:\Games` and the mod folder
//! `D:\DownloadedMods`, cut to the keys that the search reads and a few others.

use std::fs;
use std::path::{Path, PathBuf};

/// The text of the sample files.
pub const LIBRARY_FOLDERS: &str = include_str!("samples/libraryfolders.vdf");
pub const APP_MANIFEST: &str = include_str!("samples/appmanifest_1874880.acf");
pub const LOCAL_CONFIG: &str = include_str!("samples/localconfig.vdf");

/// Makes a Steam folder `steam` with the game installed, and the user `12345678` whose launch
/// options name `mods`. `steam` is the library in place of `D:\Games`, and `mods` is the mod
/// folder in place of `D:\DownloadedMods`. Returns the game folder.
pub fn write_sample_steam(steam: &Path, mods: &Path) -> PathBuf {
    let game = steam.join("steamapps/common/Arma Reforger");
    fs::create_dir_all(&game).unwrap();
    fs::write(
        steam.join("steamapps/libraryfolders.vdf"),
        LIBRARY_FOLDERS.replace(r"D:\\Games", &escaped(steam)),
    )
    .unwrap();
    fs::write(
        steam.join("steamapps/appmanifest_1874880.acf"),
        APP_MANIFEST,
    )
    .unwrap();
    write_local_config(
        steam,
        &LOCAL_CONFIG.replace(r"D:\\DownloadedMods", &escaped(mods)),
    );
    game
}

/// Writes `text` as the `localconfig.vdf` of the user `12345678`.
pub fn write_local_config(steam: &Path, text: &str) {
    let config = steam.join("userdata/12345678/config");
    fs::create_dir_all(&config).unwrap();
    fs::write(config.join("localconfig.vdf"), text).unwrap();
}

/// A path as Steam writes it in a quoted text: each `\` doubled.
fn escaped(path: &Path) -> String {
    path.display().to_string().replace('\\', r"\\")
}
