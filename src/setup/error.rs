//! Errors of the setup.

use std::fmt;
use std::path::PathBuf;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Neither `CLAUDE_CONFIG_DIR` nor the home folder is known.
    NoHome,

    /// The settings file exists but cannot be read.
    Read { file: PathBuf, err: std::io::Error },

    /// The settings file is not JSON.
    NotJson {
        file: PathBuf,
        err: serde_json::Error,
    },

    /// The settings file is JSON, but not an object.
    NotObject { file: PathBuf },

    /// The `env` of the settings file is not an object.
    EnvNotObject { file: PathBuf },

    /// The settings file cannot be written.
    Write { file: PathBuf, err: std::io::Error },

    // -- Externals
    /// The terminal cannot be read or written.
    Terminal(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHome => write!(
                f,
                "The setup cannot find the settings file of Claude Code: CLAUDE_CONFIG_DIR is not \
                 set, and the home folder is not known. Set CLAUDE_CONFIG_DIR to the folder of \
                 settings.json and run the setup again."
            ),
            Self::Read { file, err } => write!(
                f,
                "The setup cannot read {}: {err}. Nothing was written.",
                file.display()
            ),
            Self::NotJson { file, err } => write!(
                f,
                "{} is not valid JSON ({err}). Fix the file, then run the setup again. Nothing \
                 was written.",
                file.display()
            ),
            Self::NotObject { file } => write!(
                f,
                "{} is not a JSON object {{ ... }}. Fix the file, then run the setup again. \
                 Nothing was written.",
                file.display()
            ),
            Self::EnvNotObject { file } => write!(
                f,
                "\"env\" in {} is not a JSON object {{ ... }}. Fix the file, then run the setup \
                 again. Nothing was written.",
                file.display()
            ),
            Self::Write { file, err } => write!(
                f,
                "The setup cannot write {}: {err}. The file did not change.",
                file.display()
            ),
            Self::Terminal(err) => write!(f, "The setup cannot use the terminal: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { err, .. } | Self::Write { err, .. } | Self::Terminal(err) => Some(err),
            Self::NotJson { err, .. } => Some(err),
            Self::NoHome | Self::NotObject { .. } | Self::EnvNotObject { .. } => None,
        }
    }
}

// region:    --- Froms

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Terminal(err)
    }
}

// endregion: --- Froms
