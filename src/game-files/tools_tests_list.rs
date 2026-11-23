//! The tests of `game_list`.

use std::path::PathBuf;

use serde_json::json;

use super::*;

// -- Test Support

/// A game folder with `CAMERA_FILES`, 4 files of 13 bytes each, and a profile folder with the mod
/// tvtLobby: `Scripts/Big.c`, compressed, with a full size of 1000 bytes, and `Scripts/Small.c`,
/// 9 bytes.
fn game_and_mod(test: &str) -> (PathBuf, PathBuf) {
    let game = game_folder(test, CAMERA_FILES);
    let profile = game.with_file_name("profile");
    let pak = PakBuilder::new()
        .dir("", 1)
        .dir("Scripts", 2)
        .compressed_file("Big.c", &[b'a'; 1000])
        .file("Small.c", b"// small\n");
    write_built_pak(
        &profile.join("addons/tvtLobby_5F2A6A1D2A3B4C5D/data.pak"),
        &pak,
    );
    (game, profile)
}

// -- Tests

#[tokio::test]
async fn lists_the_game_and_each_downloaded_mod_with_its_id_file_count_and_size() {
    // -- Setup & Fixtures
    let (game, profile) =
        game_and_mod("lists_the_game_and_each_downloaded_mod_with_its_id_file_count_and_size");

    // -- Exec
    let answer = list_in(settings(&game, Some(&profile)), json!({})).await;

    // -- Check
    assert!(!answer.is_error, "{}", answer.text);
    assert_eq!(
        answer.text,
        "The game and 1 downloaded mod:\n\
         - game: 4 files, 52 bytes\n\
         - tvtLobby (ID 5F2A6A1D2A3B4C5D): 2 files, 1009 bytes\n\
         To list the files of one of them with the size of each, give its name as source."
    );
}

#[tokio::test]
async fn lists_the_files_of_one_mod_with_their_full_size() {
    // -- Setup & Fixtures
    let (game, profile) = game_and_mod("lists_the_files_of_one_mod_with_their_full_size");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "source": "tvtLobby" }),
    )
    .await;

    // -- Check
    // The size of Big.c is its full size, not its compressed size in the .pak file.
    assert!(!answer.is_error, "{}", answer.text);
    assert_eq!(
        answer.text,
        "mod tvtLobby has 2 files, 1009 bytes in all:\n\
         - Scripts/Big.c (1000 bytes)\n\
         - Scripts/Small.c (9 bytes)"
    );
}

#[tokio::test]
async fn finds_a_mod_by_its_id_in_any_capital_letters() {
    // -- Setup & Fixtures
    let (game, profile) = game_and_mod("finds_a_mod_by_its_id_in_any_capital_letters");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "source": "5f2a6a1d2a3b4c5d" }),
    )
    .await;

    // -- Check
    assert!(
        answer
            .text
            .starts_with("mod tvtLobby has 2 files, 1009 bytes in all:"),
        "{}",
        answer.text
    );
}

#[tokio::test]
async fn counts_only_the_files_with_the_path_part_in_each_source() {
    // -- Setup & Fixtures
    let (game, profile) = game_and_mod("counts_only_the_files_with_the_path_part_in_each_source");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "path": "Scripts\\" }),
    )
    .await;

    // -- Check
    // Three of the four game files are in scripts/, and both mod files are in Scripts/.
    assert_eq!(
        answer.text,
        "The files with \"scripts/\" in their path, in the game and 1 downloaded mod:\n\
         - game: 3 files, 39 bytes\n\
         - tvtLobby (ID 5F2A6A1D2A3B4C5D): 2 files, 1009 bytes\n\
         To list the files of one of them with the size of each, give its name as source."
    );
}

#[tokio::test]
async fn lists_the_files_of_the_game_with_the_path_part() {
    // -- Setup & Fixtures
    let (game, profile) = game_and_mod("lists_the_files_of_the_game_with_the_path_part");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "source": "game", "path": "Character/" }),
    )
    .await;

    // -- Check
    assert_eq!(
        answer.text,
        "game has 2 files with \"character/\" in their path, 26 bytes in all:\n\
         - scripts/Game/Character/CharacterCameraHandlerComponent.c (13 bytes)\n\
         - scripts/Game/Plugins/Persistence/System/Serializers/Components/Character/\
         SCR_CharacterCameraHandlerComponentSerializer.c (13 bytes)"
    );
}

#[tokio::test]
async fn lists_only_the_limit_and_says_how_many_files_the_source_has() {
    // -- Setup & Fixtures
    let (game, profile) =
        game_and_mod("lists_only_the_limit_and_says_how_many_files_the_source_has");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "source": "game", "limit": 1 }),
    )
    .await;

    // -- Check
    assert_eq!(
        answer.text,
        "game has 4 files, 52 bytes in all. The first 1 come below, in path order. Give a \
         part of the path to narrow the list, or a larger limit (at most 100).\n\
         - Prefabs/Characters/Core/Character_Base.et (13 bytes)"
    );
}

#[tokio::test]
async fn names_the_real_sources_when_the_source_is_unknown() {
    // -- Setup & Fixtures
    let (game, profile) = game_and_mod("names_the_real_sources_when_the_source_is_unknown");

    // -- Exec
    let answer = list_in(
        settings(&game, Some(&profile)),
        json!({ "source": "tvtLobbyy" }),
    )
    .await;

    // -- Check
    assert!(answer.is_error);
    assert_eq!(
        answer.text,
        "No source is named \"tvtLobbyy\". Give game, or the name or ID of a downloaded mod: \
         tvtLobby (5F2A6A1D2A3B4C5D). Leave source out to list them with their sizes."
    );
}
