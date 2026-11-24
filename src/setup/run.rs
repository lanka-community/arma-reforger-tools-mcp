//! `reforger setup`: finds the game folder and the folder of the downloaded mods through Steam,
//! shows them, and writes them to the settings file of Claude Code only when the user types `y`.
//! The server never writes the settings file and never uses a found path (ADR 0007).

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use super::settings_file::{Change, plan, settings_file, write};
use super::{Error, Result};
use crate::steam::{Found, ModsFrom, SteamSearch, find};
use crate::workbench_documents::{DOCUMENTS_SETTING, workbench_documents};

/// The setting of the game folder.
const GAME_SETTING: &str = "REFORGER_GAME_PATH";

/// The setting of the folder that holds `addons` with the downloaded mods.
const MODS_SETTING: &str = "REFORGER_WORKBENCH_PROFILE_PATH";

/// Where the setup looks, and the file it writes.
pub struct Context {
    pub search: SteamSearch,

    /// The Workbench folder in Documents.
    pub documents: Option<PathBuf>,
    pub settings_file: PathBuf,
}

/// The command that runs the setup with this program, for example
/// `"/home/me/arma-reforger-tools-mcp/target/release/reforger" setup`.
pub fn command() -> String {
    std::env::current_exe().map_or_else(
        |_| "reforger setup".to_owned(),
        |exe| format!("\"{}\" setup", exe.display()),
    )
}

/// Runs the setup with the Steam folder of the registry, the Documents folder of the server
/// settings, and the settings file of Claude Code.
pub fn run_from_env(input: &mut impl BufRead, out: &mut impl Write) -> Result<()> {
    let documents_setting = std::env::var(DOCUMENTS_SETTING)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let context = Context {
        search: SteamSearch::Registry,
        documents: workbench_documents(documents_setting.as_deref()),
        settings_file: settings_file().ok_or(Error::NoHome)?,
    };
    run(&context, input, out)
}

/// Runs the setup: shows what it read and found, shows the changes, asks, and writes only after
/// the answer `y` or `yes`. Any other answer, or the end of the input, writes nothing.
pub fn run(context: &Context, input: &mut impl BufRead, out: &mut impl Write) -> Result<()> {
    let found = find(&context.search, context.documents.as_deref());
    writeln!(out, "The setup read these places:")?;
    for line in &found.checked {
        writeln!(out, "- {line}")?;
    }
    writeln!(out)?;
    let values = show_found(&found, out)?;
    let file = &context.settings_file;
    if values.len() < 2 {
        show_by_hand(&found, file, out)?;
    }
    if values.is_empty() {
        return Ok(());
    }
    let plan = plan(file, &values)?;
    if plan.changes.is_empty() {
        writeln!(
            out,
            "{} already has these values. Nothing to write.",
            file.display()
        )?;
        return Ok(());
    }
    writeln!(out, "Changes to \"env\" in {}:", file.display())?;
    for change in &plan.changes {
        show_change(change, out)?;
    }
    write!(
        out,
        "Write these changes? Nothing else in the file changes. [y/N] "
    )?;
    out.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        writeln!(out, "\nNothing was written.")?;
        return Ok(());
    }
    write(file, &plan)?;
    writeln!(
        out,
        "Written to {}. Restart Claude Code to use the new settings.",
        file.display()
    )?;
    Ok(())
}

// region:    --- Support

/// Shows the found folders, and returns them as setting values.
fn show_found(found: &Found, out: &mut impl Write) -> Result<Vec<(&'static str, String)>> {
    let mut values = Vec::new();
    if let Some(game) = &found.game {
        values.push((GAME_SETTING, game.display().to_string()));
    }
    if let Some(mods) = &found.mods {
        values.push((MODS_SETTING, mods.folder.display().to_string()));
    }
    if values.is_empty() {
        writeln!(out, "The setup found neither the game nor the mod folder.")?;
        return Ok(values);
    }
    writeln!(out, "Found:")?;
    match &found.game {
        Some(game) => writeln!(out, "- {GAME_SETTING}: {}", game.display())?,
        None => writeln!(out, "- {GAME_SETTING}: not found")?,
    }
    match &found.mods {
        Some(mods) => writeln!(
            out,
            "- {MODS_SETTING}: {} ({})",
            mods.folder.display(),
            match mods.from {
                ModsFrom::LaunchOptions => "from -addonDownloadDir in the Steam launch options",
                ModsFrom::Documents => {
                    "the Workbench folder in Documents, because the Steam launch options have no -addonDownloadDir"
                }
            }
        )?,
        None => writeln!(out, "- {MODS_SETTING}: not found")?,
    }
    writeln!(out)?;
    Ok(values)
}

/// Says how to set the settings that the setup did not find by hand.
fn show_by_hand(found: &Found, file: &Path, out: &mut impl Write) -> Result<()> {
    writeln!(
        out,
        "Set the missing settings by hand in the \"env\" object of {}, then restart Claude Code:",
        file.display()
    )?;
    if found.game.is_none() {
        writeln!(
            out,
            "- {GAME_SETTING}: the game folder, the folder that holds ArmaReforger.exe and \
             addons, for example \"D:\\\\SteamLibrary\\\\steamapps\\\\common\\\\Arma Reforger\"."
        )?;
    }
    if found.mods.is_none() {
        writeln!(
            out,
            "- {MODS_SETTING}: the folder that holds addons with the downloaded Workshop mods, \
             for example \"D:\\\\DownloadedMods\"."
        )?;
    }
    writeln!(
        out,
        "A path works in Windows form (D:\\Games\\...) and in WSL form (/mnt/d/Games/...). In \
         JSON, write each \\ as \\\\.\n"
    )?;
    Ok(())
}

fn show_change(change: &Change, out: &mut impl Write) -> Result<()> {
    match &change.old {
        Some(old) => writeln!(out, "- {}: \"{old}\" -> \"{}\"", change.key, change.new)?,
        None => writeln!(out, "- {}: not set -> \"{}\"", change.key, change.new)?,
    }
    Ok(())
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;

// endregion: --- Tests
