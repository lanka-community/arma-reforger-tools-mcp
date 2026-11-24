//! The tests of `reforger setup`. Each test writes a settings file in its own folder in
//! `tests/.out`, never the real `~/.claude/settings.json`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::steam::sample_steam::write_sample_steam;

/// A settings file with other keys, in the format that Claude Code writes.
const OTHER_SETTINGS: &str = r#"{
  "model": "opus",
  "env": {
    "MAX_THINKING_TOKENS": "8000"
  },
  "permissions": {
    "allow": [
      "Bash(cargo:*)"
    ]
  }
}
"#;

#[test]
fn writes_the_found_folders_after_yes_and_keeps_all_other_keys() {
    // -- Setup & Fixtures
    let test = Test::new("writes_the_found_folders_after_yes_and_keeps_all_other_keys");
    fs::write(&test.context.settings_file, OTHER_SETTINGS).unwrap();

    // -- Exec
    let out = test.run("y\n");

    // -- Check
    assert!(
        out.contains(&format!(
            "Found:\n- REFORGER_GAME_PATH: {}\n- REFORGER_WORKBENCH_PROFILE_PATH: {} (from \
             -addonDownloadDir in the Steam launch options)\n",
            test.game.display(),
            test.mods.display()
        )),
        "{out}"
    );
    assert!(out.contains("- REFORGER_GAME_PATH: not set -> \""), "{out}");
    assert_eq!(
        fs::read_to_string(&test.context.settings_file).unwrap(),
        format!(
            r#"{{
  "model": "opus",
  "env": {{
    "MAX_THINKING_TOKENS": "8000",
    "REFORGER_GAME_PATH": "{}",
    "REFORGER_WORKBENCH_PROFILE_PATH": "{}"
  }},
  "permissions": {{
    "allow": [
      "Bash(cargo:*)"
    ]
  }}
}}
"#,
            test.game.display(),
            test.mods.display()
        )
    );
}

#[test]
fn shows_the_found_folders_and_writes_nothing_without_yes() {
    // -- Setup & Fixtures
    let test = Test::new("shows_the_found_folders_and_writes_nothing_without_yes");
    fs::write(&test.context.settings_file, OTHER_SETTINGS).unwrap();

    // -- Exec & Check
    for answer in ["n\n", "\n", "", "write it\n"] {
        let out = test.run(answer);
        assert!(out.contains("[y/N] "), "{out}");
        assert!(out.ends_with("Nothing was written.\n"), "{out}");
        assert_eq!(
            fs::read_to_string(&test.context.settings_file).unwrap(),
            OTHER_SETTINGS
        );
    }
}

#[test]
fn creates_the_settings_file_when_it_is_missing() {
    // -- Setup & Fixtures
    let test = Test::new("creates_the_settings_file_when_it_is_missing");

    // -- Exec
    test.run("yes\n");

    // -- Check
    let written = fs::read_to_string(&test.context.settings_file).unwrap();
    assert!(
        written.starts_with("{\n  \"env\": {\n    \"REFORGER_GAME_PATH\""),
        "{written}"
    );
}

#[test]
fn says_which_files_it_read_and_how_to_set_the_settings_when_it_finds_nothing() {
    // -- Setup & Fixtures
    let out_folder =
        out_folder("says_which_files_it_read_and_how_to_set_the_settings_when_it_finds_nothing");
    let context = Context {
        search: SteamSearch::Folder(out_folder.display().to_string()),
        documents: None,
        settings_file: out_folder.join("settings.json"),
    };

    // -- Exec
    let out = run_with(&context, "y\n");

    // -- Check
    assert!(
        out.starts_with(&format!(
            "The setup read these places:\n- {}: not found\n- The Workbench folder in Documents: \
             not found\n\nThe setup found neither the game nor the mod folder.\nSet the missing \
             settings by hand in the \"env\" object of {}, then restart Claude Code:\n- \
             REFORGER_GAME_PATH: the game folder",
            out_folder.join("steamapps/libraryfolders.vdf").display(),
            context.settings_file.display()
        )),
        "{out}"
    );
    assert!(out.contains("- REFORGER_WORKBENCH_PROFILE_PATH: the folder that holds addons"));
    assert!(!out.contains("[y/N]"), "{out}");
    assert!(!context.settings_file.exists());
}

#[test]
fn says_that_nothing_changes_when_the_settings_have_the_found_folders() {
    // -- Setup & Fixtures
    let test = Test::new("says_that_nothing_changes_when_the_settings_have_the_found_folders");
    test.run("y\n");
    let written = fs::read_to_string(&test.context.settings_file).unwrap();

    // -- Exec
    let out = test.run("");

    // -- Check
    assert!(
        out.ends_with("already has these values. Nothing to write.\n"),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(&test.context.settings_file).unwrap(),
        written
    );
}

#[test]
fn shows_the_old_value_of_a_setting_that_changes() {
    // -- Setup & Fixtures
    let test = Test::new("shows_the_old_value_of_a_setting_that_changes");
    fs::write(
        &test.context.settings_file,
        r#"{ "env": { "REFORGER_GAME_PATH": "D:\\Old" } }"#,
    )
    .unwrap();

    // -- Exec
    let out = test.run("n\n");

    // -- Check
    assert!(
        out.contains(&format!(
            "- REFORGER_GAME_PATH: \"D:\\Old\" -> \"{}\"",
            test.game.display()
        )),
        "{out}"
    );
}

#[test]
fn gives_an_error_and_writes_nothing_when_the_settings_file_is_not_json() {
    // -- Setup & Fixtures
    let test = Test::new("gives_an_error_and_writes_nothing_when_the_settings_file_is_not_json");
    fs::write(&test.context.settings_file, "{ \"env\": ").unwrap();

    // -- Exec
    let error = run(&test.context, &mut "y\n".as_bytes(), &mut Vec::new()).unwrap_err();

    // -- Check
    assert!(
        error.to_string().starts_with(&format!(
            "{} is not valid JSON",
            test.context.settings_file.display()
        )),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(&test.context.settings_file).unwrap(),
        "{ \"env\": "
    );
}

#[test]
#[cfg(unix)]
fn writes_the_file_behind_a_link_and_keeps_the_link() {
    // -- Setup & Fixtures
    let test = Test::new("writes_the_file_behind_a_link_and_keeps_the_link");
    let real = test.folder.join("config/settings.json");
    fs::create_dir_all(real.parent().unwrap()).unwrap();
    fs::write(&real, OTHER_SETTINGS).unwrap();
    std::os::unix::fs::symlink(&real, &test.context.settings_file).unwrap();

    // -- Exec
    test.run("y\n");

    // -- Check
    assert!(
        fs::symlink_metadata(&test.context.settings_file)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&real)
            .unwrap()
            .contains("REFORGER_GAME_PATH")
    );
}

// region:    --- Test Support

/// A sample Steam folder with the game and the mod folder, and a settings file that does not
/// exist yet.
struct Test {
    folder: PathBuf,
    game: PathBuf,
    mods: PathBuf,
    context: Context,
}

impl Test {
    fn new(test: &str) -> Self {
        let folder = out_folder(test);
        let steam = folder.join("Games");
        let mods = folder.join("DownloadedMods");
        fs::create_dir_all(mods.join("addons")).unwrap();
        let game = write_sample_steam(&steam, &mods);
        let context = Context {
            search: SteamSearch::Folder(steam.display().to_string()),
            documents: None,
            settings_file: folder.join("settings.json"),
        };
        Self {
            folder,
            game,
            mods,
            context,
        }
    }

    fn run(&self, answer: &str) -> String {
        run_with(&self.context, answer)
    }
}

/// Runs the setup with `answer` as the input, and returns its output.
fn run_with(context: &Context, answer: &str) -> String {
    let mut out = Vec::new();
    run(context, &mut answer.as_bytes(), &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// An empty folder for the files of one test: `tests/.out/<test>`.
fn out_folder(test: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/.out")
        .join(test);
    assert!(folder.to_string_lossy().contains("tests/.out"));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

// endregion: --- Test Support
