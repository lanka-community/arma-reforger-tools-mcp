//! Errors of the Steam search.

use std::fmt;
use std::path::PathBuf;

use super::vdf::Damage;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A Steam file cannot be read.
    Read { file: PathBuf, err: std::io::Error },

    /// A Steam file is not in the `KeyValues` format.
    Damaged { file: PathBuf, damage: Damage },

    /// `libraryfolders.vdf` lists no library with the app.
    AppNotInLibrary { file: PathBuf, app: u32 },

    /// The app manifest has no `installdir`.
    NoInstallDir { file: PathBuf },

    /// The folder of the app does not exist.
    NoAppFolder { folder: PathBuf },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { file, err } => write!(f, "{} cannot be read: {err}", file.display()),
            Self::Damaged { file, damage } => {
                write!(f, "{} is damaged: {damage}", file.display())
            }
            Self::AppNotInLibrary { file, app } => {
                write!(f, "{} lists no library with app {app}", file.display())
            }
            Self::NoInstallDir { file } => {
                write!(f, "{} has no \"installdir\"", file.display())
            }
            Self::NoAppFolder { folder } => write!(f, "{} does not exist", folder.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { err, .. } => Some(err),
            Self::Damaged { .. }
            | Self::AppNotInLibrary { .. }
            | Self::NoInstallDir { .. }
            | Self::NoAppFolder { .. } => None,
        }
    }
}
