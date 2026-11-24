//! The setup of the settings: `reforger setup` finds the game folder and the folder of the
//! downloaded mods through Steam, and writes them to the settings file of Claude Code after the
//! user confirms.

// region:    --- Modules

mod error;
mod run;
mod settings_file;

pub use error::{Error, Result};
pub use run::{command, run_from_env};

// endregion: --- Modules
