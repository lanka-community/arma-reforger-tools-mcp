//! Deletes the Handler script from a mod project, so that Workbench does not pack it with the
//! mod. It deletes only the folder of the bridge, and then the folders above it that are empty
//! now. It makes no NET API call, so it works when Workbench is closed.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::install::handler_folder;
use super::{Error, Result};

/// Deletes the folder of the bridge in the project folder `project`, with everything in it. Then
/// it deletes `Scripts/WorkbenchGame` and `Scripts` when they are empty now. False: the folder of
/// the bridge was not there, so it deleted nothing.
pub(super) fn cleanup(project: &Path) -> Result<bool> {
    let folder = handler_folder(project);
    match fs::remove_dir_all(&folder) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(Error::CleanupFailed { path: folder, err }),
    }
    for parent in folder
        .ancestors()
        .skip(1)
        .take_while(|parent| *parent != project)
    {
        let failed = |err| Error::CleanupFailed {
            path: parent.to_owned(),
            err,
        };
        // A folder that holds a file or a folder stays, and so do the folders above it.
        if fs::read_dir(parent).map_err(failed)?.next().is_some() {
            break;
        }
        fs::remove_dir(parent).map_err(failed)?;
    }
    Ok(true)
}
