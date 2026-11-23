//! The `game_list` Tool: lists the game and the downloaded mods with their file counts and sizes,
//! or the files of one of them with the full size of each.

use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::Deserialize;

use super::index::{GameFile, Index, Source};
use crate::server::Answer;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Deserialize, JsonSchema)]
pub(super) struct ListInput {
    /// The source whose files to list, each with its size: `game`, or the name or ID of a
    /// downloaded mod, for example `tvtLobby` or `5F2A6A1D2A3B4C5D`. Without it, the answer lists
    /// the game and every downloaded mod with its file count and size.
    source: Option<String>,

    /// Count and list only the files with this part in their path, for example `scripts/` for
    /// the scripts. Capital letters do not matter.
    path: Option<String>,

    /// How many files of the source to list, from 1 to 100, for example 50. The default is 20.
    #[schemars(range(min = 1, max = 100))]
    limit: Option<usize>,
}

/// Answers one `game_list` call.
pub(super) fn list(index: &Index, input: &ListInput) -> Answer {
    let path = input
        .path
        .as_deref()
        .map(|path| path.trim().replace('\\', "/").to_lowercase())
        .filter(|path| !path.is_empty());
    let has_path = |file: &&GameFile| {
        path.as_deref()
            .is_none_or(|path| file.path.to_lowercase().contains(path))
    };
    let source = input
        .source
        .as_deref()
        .map(str::trim)
        .filter(|source| !source.is_empty());
    let mut text = match source {
        None => sources(index, path.as_deref(), has_path),
        Some(wanted) => {
            let Some(source) = (0..index.sources.len()).find(|&s| index.is_source(s, wanted))
            else {
                return Err(unknown_source(index, wanted));
            };
            let files: Vec<&GameFile> = index
                .files
                .iter()
                .filter(|file| file.source == source)
                .filter(has_path)
                .collect();
            let limit = input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
            files_of(&index.source_name(source), path.as_deref(), files, limit)
        }
    };
    for note in &index.notes {
        let _ = write!(text, "\n\n{note}");
    }
    Ok(text)
}

// region:    --- Support

/// The game and each mod, with the count and the full size of its files that `has_path` keeps.
fn sources(index: &Index, path: Option<&str>, has_path: impl Fn(&&GameFile) -> bool) -> String {
    let mut counts = vec![(0_usize, 0_u64); index.sources.len()];
    for file in index.files.iter().filter(has_path) {
        if let Some((count, bytes)) = counts.get_mut(file.source) {
            *count += 1;
            *bytes += u64::from(file.len());
        }
    }
    let mods = index.sources.len().saturating_sub(1);
    let covered = match mods {
        1 => "game and 1 downloaded mod".to_owned(),
        _ => format!("game and {mods} downloaded mods"),
    };
    let mut text = match path {
        None => format!("The {covered}:\n"),
        Some(path) => format!("The files with \"{path}\" in their path, in the {covered}:\n"),
    };
    for (Source { name, id }, (count, bytes)) in index.sources.iter().zip(counts) {
        let id = id.as_deref().map_or_default(|id| format!(" (ID {id})"));
        let _ = writeln!(text, "- {name}{id}: {count} files, {bytes} bytes");
    }
    text.push_str(
        "To list the files of one of them with the size of each, give its name as source.",
    );
    text
}

/// The `files` of the source `name` in path order, at most `limit`, each with its full size.
fn files_of(name: &str, path: Option<&str>, mut files: Vec<&GameFile>, limit: usize) -> String {
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let bytes: u64 = files.iter().map(|file| u64::from(file.len())).sum();
    let with_path = path.map_or_default(|path| format!(" with \"{path}\" in their path"));
    let mut text = format!(
        "{name} has {} files{with_path}, {bytes} bytes in all",
        files.len()
    );
    if files.len() > limit {
        let _ = writeln!(
            text,
            ". The first {limit} come below, in path order. Give a part of the path to narrow the \
             list, or a larger limit (at most {MAX_LIMIT})."
        );
    } else {
        text.push_str(":\n");
    }
    for file in files.iter().take(limit) {
        let _ = writeln!(text, "- {} ({} bytes)", file.path, file.len());
    }
    text.trim_end().to_owned()
}

/// The hint for a source that is not the game and no downloaded mod.
fn unknown_source(index: &Index, wanted: &str) -> String {
    let mods: Vec<String> = index
        .sources
        .iter()
        .skip(1)
        .map(|Source { name, id }| match id {
            Some(id) => format!("{name} ({id})"),
            None => name.clone(),
        })
        .collect();
    let mods = if mods.is_empty() {
        "none is downloaded".to_owned()
    } else {
        mods.join(", ")
    };
    format!(
        "No source is named \"{wanted}\". Give game, or the name or ID of a downloaded mod: \
         {mods}. Leave source out to list them with their sizes."
    )
}

// endregion: --- Support
