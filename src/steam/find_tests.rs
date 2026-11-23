//! The tests of the Steam search. They read the sample Steam files in `samples/`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::steam::sample_steam::{LOCAL_CONFIG, write_local_config, write_sample_steam};

#[test]
fn finds_the_game_in_a_steam_folder_that_is_not_the_default() {
    // -- Setup & Fixtures
    let out = out_folder("finds_the_game_in_a_steam_folder_that_is_not_the_default");
    let steam = out.join("Games");
    let game = write_sample_steam(&steam, &mods_folder(&out));

    // -- Exec
    let found = find(&folder(&steam), None);

    // -- Check
    assert_eq!(found.game, Some(game.clone()));
    assert_eq!(find_game(&folder(&steam)), Some(game.clone()));
    assert_eq!(
        found.checked.first().unwrap(),
        &format!(
            "{}: the game (app 1874880) is in {}",
            steam.join("steamapps/libraryfolders.vdf").display(),
            game.display()
        )
    );
}

#[test]
fn finds_the_mod_folder_in_the_launch_options_of_the_game() {
    // -- Setup & Fixtures
    let out = out_folder("finds_the_mod_folder_in_the_launch_options_of_the_game");
    let steam = out.join("Games");
    let mods = mods_folder(&out);
    write_sample_steam(&steam, &mods);
    let documents = out.join("Documents");
    fs::create_dir_all(&documents).unwrap();

    // -- Exec
    let found = find(&folder(&steam), Some(&documents));

    // -- Check
    assert_eq!(
        found.mods,
        Some(Mods {
            folder: mods.clone(),
            from: ModsFrom::LaunchOptions
        })
    );
    assert!(
        found.checked.contains(&format!(
            "{}: -addonDownloadDir \"{}\"",
            steam
                .join("userdata/12345678/config/localconfig.vdf")
                .display(),
            mods.display()
        )),
        "{:#?}",
        found.checked
    );
}

#[test]
fn uses_the_documents_folder_when_the_launch_options_name_no_mod_folder() {
    // -- Setup & Fixtures
    let out = out_folder("uses_the_documents_folder_when_the_launch_options_name_no_mod_folder");
    let steam = out.join("Games");
    write_sample_steam(&steam, &mods_folder(&out));
    write_local_config(
        &steam,
        &LOCAL_CONFIG.replace("-addonDownloadDir", "-noSplash"),
    );
    let documents = out.join("Documents");
    fs::create_dir_all(&documents).unwrap();

    // -- Exec
    let found = find(&folder(&steam), Some(&documents));

    // -- Check
    assert_eq!(
        found.mods,
        Some(Mods {
            folder: documents,
            from: ModsFrom::Documents
        })
    );
    assert!(
        found
            .checked
            .iter()
            .any(|line| line.ends_with("the launch options of the game have no -addonDownloadDir")),
        "{:#?}",
        found.checked
    );
}

#[test]
fn names_each_file_it_read_when_it_finds_nothing() {
    // -- Setup & Fixtures
    let out = out_folder("names_each_file_it_read_when_it_finds_nothing");

    // -- Exec
    let found = find(&folder(&out), None);

    // -- Check
    assert_eq!(found.game, None);
    assert_eq!(found.mods, None);
    assert_eq!(
        found.checked,
        [
            format!(
                "{}: not found",
                out.join("steamapps/libraryfolders.vdf").display()
            ),
            "The Workbench folder in Documents: not found".to_owned()
        ]
    );
}

#[test]
fn gives_an_error_line_not_a_crash_for_a_damaged_library_file() {
    // -- Setup & Fixtures
    let out = out_folder("gives_an_error_line_not_a_crash_for_a_damaged_library_file");
    let steam = out.join("Games");
    write_sample_steam(&steam, &mods_folder(&out));
    let library = steam.join("steamapps/libraryfolders.vdf");
    let text = fs::read_to_string(&library).unwrap();
    fs::write(&library, text.trim_end().trim_end_matches('}')).unwrap();

    // -- Exec
    let found = find(&folder(&steam), None);

    // -- Check
    assert_eq!(found.game, None);
    assert_eq!(
        found.checked.first().unwrap(),
        &format!(
            "{} is damaged: line 2: the block \"libraryfolders\" has no closing }}",
            library.display()
        )
    );
}

#[test]
fn uses_the_documents_folder_when_the_launch_options_file_is_damaged() {
    // -- Setup & Fixtures
    let out = out_folder("uses_the_documents_folder_when_the_launch_options_file_is_damaged");
    let steam = out.join("Games");
    write_sample_steam(&steam, &mods_folder(&out));
    write_local_config(&steam, "\"UserLocalConfigStore\"\n{\n\t\"Software");
    let documents = out.join("Documents");
    fs::create_dir_all(&documents).unwrap();

    // -- Exec
    let found = find(&folder(&steam), Some(&documents));

    // -- Check
    assert_eq!(found.mods.unwrap().from, ModsFrom::Documents);
    assert!(
        found.checked.contains(&format!(
            "{} is damaged: line 3: a text in quotes has no closing quote",
            steam
                .join("userdata/12345678/config/localconfig.vdf")
                .display()
        )),
        "{:#?}",
        found.checked
    );
}

#[test]
fn reads_the_steam_folder_from_the_output_of_reg_exe() {
    // -- Setup & Fixtures
    // The output of `reg.exe query HKCU\Software\Valve\Steam /v SteamPath` on Windows 11.
    let output = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    d:/games\r\n\r\n";

    // -- Exec & Check
    assert_eq!(registry_value(output).as_deref(), Some("d:/games"));
    assert_eq!(
        registry_value(
            "ERROR: The system was unable to find the specified registry key or value.\r\n"
        ),
        None
    );
}

#[test]
fn reads_the_mod_folder_with_and_without_quotes() {
    // -- Exec & Check
    for (options, value) in [
        (
            r#"-addonDownloadDir "D:\Downloaded Mods" -addonsDir "D:\x""#,
            Some(r"D:\Downloaded Mods"),
        ),
        (r"-noSplash -addondownloaddir D:\Mods", Some(r"D:\Mods")),
        ("-addonDownloadDir", None),
        ("-addonDownloadDir -noSplash", None),
        ("", None),
    ] {
        assert_eq!(option_value(options).as_deref(), value, "{options}");
    }
}

// region:    --- Test Support

fn folder(steam: &Path) -> SteamSearch {
    SteamSearch::Folder(steam.display().to_string())
}

/// The mod folder `<out>/DownloadedMods`, made on disk.
fn mods_folder(out: &Path) -> PathBuf {
    let mods = out.join("DownloadedMods");
    fs::create_dir_all(mods.join("addons")).unwrap();
    mods
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
