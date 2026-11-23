//! Finds the mod that Claude named in the `mod` input of `workbench_launch` and
//! `workbench_cleanup`, in the project list of Workbench.
//!
//! A lookup has two kinds of failure. A failure to read the project list is a case of `Error`.
//! A `mod` input that names no mod, or that is empty when the Tool needs a mod, is a wrong input
//! from Claude: it is a `WrongMod`, and the Tool answers its text as a hint (`docs/agents/rust.md`,
//! Errors).

use std::fmt;

use super::bridge::Bridge;
use super::error::NoProject;
use super::project::{Project, listed_mods};
use super::{Error, Result};

/// The result of a lookup when the project list was read: the mod, or the hint for a wrong
/// `mod` input.
pub(super) type Lookup<T> = core::result::Result<T, WrongMod>;

/// A `mod` input from Claude that names no mod of the project list. Each case gives the hint
/// text that the Tool answers.
#[derive(Debug)]
pub(super) enum WrongMod {
    /// The project list has no mod with the name that Claude gave to `tool`.
    Unknown {
        tool: ModTool,
        wanted: String,
        names: Vec<String>,
    },

    /// Claude gave no mod to `workbench_launch`, and the project list has more than one.
    Choose { names: Vec<String> },

    /// Claude gave an empty `mod` to `workbench_cleanup`. `names`: the mods of the project list.
    Required { names: Vec<String> },
}

/// A Tool that finds a mod in the project list by the name that Claude gave.
#[derive(Debug, Clone, Copy)]
pub(super) enum ModTool {
    Launch,
    Cleanup,
}

/// The mod for `workbench_cleanup`, named `wanted` in the project list: its ID or its folder
/// name, in any case. An unknown name gives the hint of `workbench_launch`, with
/// `workbench_cleanup` as the Tool to call again. The Tool deletes files, so it never picks a mod
/// itself: an empty name is a wrong input, also when the project list has one mod.
pub(super) fn find_mod(bridge: &Bridge, wanted: &str) -> Result<Lookup<Project>> {
    let mods = mods(bridge)?;
    let wanted = wanted.trim();
    if wanted.is_empty() {
        return Ok(Err(WrongMod::Required {
            names: names(&mods),
        }));
    }
    Ok(named(mods, wanted, ModTool::Cleanup))
}

/// The mod for `workbench_launch`: the mod named `wanted` (its ID or its folder name, in any
/// case), or the one mod when Claude named none. `None`: no mod, so Workbench starts on its
/// launcher.
pub(super) fn choose_mod(bridge: &Bridge, wanted: Option<&str>) -> Result<Lookup<Option<Project>>> {
    let mods = mods(bridge)?;
    let Some(wanted) = wanted.map(str::trim).filter(|wanted| !wanted.is_empty()) else {
        return Ok(match mods.len() {
            0 => Ok(None),
            1 => Ok(mods.into_iter().next()),
            _ => Err(WrongMod::Choose {
                names: names(&mods),
            }),
        });
    };
    Ok(named(mods, wanted, ModTool::Launch).map(Some))
}

impl fmt::Display for WrongMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { wanted, names, .. } if names.is_empty() => write!(
                f,
                "The project list of Workbench has no mod \"{wanted}\": it has no mods. Call \
                 workbench_launch without a mod to open the Workbench launcher, and create the \
                 mod there."
            ),
            Self::Unknown {
                tool,
                wanted,
                names,
            } => write!(
                f,
                "The project list of Workbench has no mod \"{wanted}\". Its mods: {}. Call \
                 {tool} again with one of them.",
                names.join(", ")
            ),
            Self::Choose { names } => write!(
                f,
                "The project list of Workbench has more than one mod: {}. Call workbench_launch \
                 again with `mod` set to one of them.",
                names.join(", ")
            ),
            Self::Required { names } if names.is_empty() => write!(
                f,
                "workbench_cleanup needs `mod`: the mod to clean up. The project list of \
                 Workbench has no mods, so no mod has a Handler script to delete. If the mod is \
                 not in the list, call workbench_launch without a mod to open the Workbench \
                 launcher, and ask the user to add the mod there (Add Existing). Then call \
                 workbench_cleanup again with `mod` set to it."
            ),
            Self::Required { names } => write!(
                f,
                "workbench_cleanup needs `mod`: the mod to clean up. The project list of \
                 Workbench has these mods: {}. Call workbench_cleanup again with `mod` set to \
                 one of them.",
                names.join(", ")
            ),
        }
    }
}

impl fmt::Display for ModTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Launch => "workbench_launch",
            Self::Cleanup => "workbench_cleanup",
        })
    }
}

// region:    --- Support

/// The mods of the project list. A Workbench that never ran has no Documents folder and no
/// project list, so it has no mods.
fn mods(bridge: &Bridge) -> Result<Vec<Project>> {
    let documents = match bridge.settings().documents() {
        Ok(documents) => documents,
        Err(Error::DocumentsNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    match listed_mods(&documents) {
        Err(Error::NoProject {
            reason: NoProject::NoList { .. },
        }) => Ok(Vec::new()),
        listed => listed,
    }
}

/// The mod named `wanted`: its ID or its folder name, in any case. An unknown name gives a hint
/// that names the mods and tells Claude to call `tool` again.
fn named(mods: Vec<Project>, wanted: &str, tool: ModTool) -> Lookup<Project> {
    let names = names(&mods);
    let is_wanted = |project: &Project| {
        project.id.eq_ignore_ascii_case(wanted)
            || project
                .folder
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
    };
    mods.into_iter()
        .find(is_wanted)
        .ok_or_else(|| WrongMod::Unknown {
            tool,
            wanted: wanted.to_owned(),
            names,
        })
}

/// The IDs of `mods`, for an answer that names them.
fn names(mods: &[Project]) -> Vec<String> {
    mods.iter().map(|project| project.id.clone()).collect()
}

// endregion: --- Support
