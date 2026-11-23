//! The user settings file of Claude Code, `~/.claude/settings.json`. The setup puts its values in
//! the `env` object of the file, and Claude Code gives that `env` to the server. Every other key,
//! and the order of all keys, stays as it was.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{Error, Result};

/// One value that the setup changes in `env`.
#[derive(Debug, PartialEq)]
pub struct Change {
    pub key: &'static str,

    /// The value before the change, or `None` when the key is not set.
    pub old: Option<String>,
    pub new: String,
}

/// The changes to the file and the full new text of the file.
pub struct Plan {
    pub changes: Vec<Change>,
    text: String,
}

/// The settings file of Claude Code: `settings.json` in `CLAUDE_CONFIG_DIR`, or in `.claude` of
/// the home folder.
pub fn settings_file() -> Option<PathBuf> {
    let folder = match std::env::var_os("CLAUDE_CONFIG_DIR").filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::home_dir()?.join(".claude"),
    };
    Some(folder.join("settings.json"))
}

/// Plans to set each key of `values` in the `env` of `file`. A missing file counts as empty. It
/// reads the file and writes nothing.
pub fn plan(file: &Path, values: &[(&'static str, String)]) -> Result<Plan> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => "{}".to_owned(),
        Err(err) => {
            return Err(Error::Read {
                file: file.to_owned(),
                err,
            });
        }
    };
    let mut settings: Value = serde_json::from_str(&text).map_err(|err| Error::NotJson {
        file: file.to_owned(),
        err,
    })?;
    let env = settings
        .as_object_mut()
        .ok_or_else(|| Error::NotObject {
            file: file.to_owned(),
        })?
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| Error::EnvNotObject {
            file: file.to_owned(),
        })?;
    let mut changes = Vec::new();
    for (key, new) in values {
        let old = env.get(*key).map(|old| match old {
            Value::String(old) => old.clone(),
            other => other.to_string(),
        });
        if old.as_deref() != Some(new.as_str()) {
            env.insert((*key).to_owned(), Value::String(new.clone()));
            changes.push(Change {
                key,
                old,
                new: new.clone(),
            });
        }
    }
    let mut text = serde_json::to_string_pretty(&settings).map_err(|err| Error::NotJson {
        file: file.to_owned(),
        err,
    })?;
    text.push('\n');
    Ok(Plan { changes, text })
}

/// Writes the planned text to `file`. When `file` is a link, it writes the file that the link
/// points to, so the link stays. It writes a temp file in the same folder first, then renames
/// it: a rename is all or nothing, so a stopped setup never leaves half a file.
pub fn write(file: &Path, plan: &Plan) -> Result<()> {
    let target = fs::canonicalize(file).unwrap_or_else(|_| file.to_owned());
    let failed = |err| Error::Write {
        file: target.clone(),
        err,
    };
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder).map_err(failed)?;
    }
    let temp = target.with_extension("json.reforger-setup.tmp");
    fs::write(&temp, &plan.text).map_err(failed)?;
    fs::rename(&temp, &target).map_err(failed)
}
