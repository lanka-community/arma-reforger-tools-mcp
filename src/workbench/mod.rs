#![doc = include_str!("README.md")]

// region:    --- Modules

mod bridge;
mod cleanup;
mod error;
mod install;
mod launch;
mod mod_lookup;
mod net;
mod project;
mod settings;
mod steam;
mod tools;
mod wire;

pub use bridge::Bridge;
use error::{Error, Result};
pub use tools::tools;

// endregion: --- Modules
