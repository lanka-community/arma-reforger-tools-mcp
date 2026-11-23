//! The Steam libraries and the folders of the apps in them. `steamapps/libraryfolders.vdf` in the
//! Steam folder names each library folder and the apps in it, and
//! `steamapps/appmanifest_<app>.acf` in a library names the folder of the app under
//! `steamapps/common`.

use std::fs;
use std::path::{Path, PathBuf};

use super::vdf::{Block, parse};
use super::{Error, Result};
use crate::host_path::host_path;

/// The libraries of one Steam folder, from its `libraryfolders.vdf`.
pub struct Libraries {
    file: PathBuf,
    top: Block,
}

/// The folder of an installed app.
#[derive(Debug, PartialEq)]
pub struct AppFolder {
    /// In Windows form, as Steam wrote the library path, for a program of Windows.
    pub windows: String,

    /// In the form of this computer (ADR 0006).
    pub here: PathBuf,
}

/// Reads a Steam file in the `KeyValues` format.
pub fn read_vdf(file: &Path) -> Result<Block> {
    let text = fs::read_to_string(file).map_err(|err| Error::Read {
        file: file.to_owned(),
        err,
    })?;
    parse(&text).map_err(|damage| Error::Damaged {
        file: file.to_owned(),
        damage,
    })
}

impl Libraries {
    /// Reads `steamapps/libraryfolders.vdf` of the Steam folder `steam` (in the form of this
    /// computer).
    pub fn read(steam: &Path) -> Result<Self> {
        let file = steam.join("steamapps/libraryfolders.vdf");
        let top = read_vdf(&file)?;
        Ok(Self { file, top })
    }

    /// The file that the libraries come from.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The folder of the app with the Steam ID `app`. Its library must list the app, its
    /// manifest must name its folder, and the folder must exist.
    pub fn app_folder(&self, app: u32) -> Result<AppFolder> {
        let id = app.to_string();
        let root = self
            .top
            .block("libraryfolders")
            .into_iter()
            .flat_map(Block::blocks)
            .find(|(_, library)| {
                library
                    .block("apps")
                    .is_some_and(|apps| apps.get(&id).is_some())
            })
            .and_then(|(_, library)| library.text("path"))
            .ok_or_else(|| Error::AppNotInLibrary {
                file: self.file.clone(),
                app,
            })?;
        let manifest = host_path(root).join(format!("steamapps/appmanifest_{app}.acf"));
        let folder = read_vdf(&manifest)?
            .block("AppState")
            .and_then(|state| state.text("installdir"))
            .map(str::to_owned)
            .ok_or(Error::NoInstallDir { file: manifest })?;
        let here = host_path(root).join("steamapps/common").join(&folder);
        if !here.is_dir() {
            return Err(Error::NoAppFolder { folder: here });
        }
        Ok(AppFolder {
            windows: format!(r"{root}\steamapps\common\{folder}"),
            here,
        })
    }
}
