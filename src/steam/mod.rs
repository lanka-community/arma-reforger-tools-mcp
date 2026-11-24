//! Finds the folders of Steam apps for every Capability and for the setup: the Steam folder, the
//! library of each app, and the launch options of the game. Steam keeps them in files of its
//! `KeyValues` format (`vdf.rs`).

// region:    --- Modules

mod error;
mod find;
mod library;
mod vdf;

#[cfg(test)]
pub mod sample_steam;

pub use error::{Error, Result};
pub use find::{Found, GAME_APP, ModsFrom, SteamSearch, find, find_game, steam_from_registry};
pub use library::Libraries;

// endregion: --- Modules
