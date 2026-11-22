//! The tests of `workbench_cleanup`. Nothing answers on the Workbench port in these tests, so
//! they also prove that the Tool works when Workbench is closed.

use std::fs;

use serde_json::json;

use super::*;

#[tokio::test]
async fn deletes_only_the_handler_folder_and_changes_no_other_file() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let project = setup.project("BridgeTest");
    let handler = setup.handler_file();
    fs::create_dir_all(handler.parent().unwrap()).unwrap();
    fs::write(&handler, "class ReforgerBridge {}").unwrap();
    fs::write(handler.with_file_name("ReforgerBridge.c.tmp"), "half").unwrap();
    fs::write(
        project.join("Scripts/WorkbenchGame/MyPlugin.c"),
        "class MyPlugin {}",
    )
    .unwrap();
    fs::create_dir_all(project.join("Scripts/Game")).unwrap();
    fs::write(project.join("Scripts/Game/MyMode.c"), "class MyMode {}").unwrap();
    let before = snapshot(&project);

    // -- Exec
    let answer = setup.cleanup(json!({ "mod": "BridgeTest" })).await;

    // -- Check
    assert!(!answer.is_error, "{}", answer.text);
    assert!(!handler.parent().unwrap().exists());
    let mut expected = before;
    expected.retain(|path, _| !path.starts_with("Scripts/WorkbenchGame/ReforgerBridge"));
    assert_eq!(snapshot(&project), expected);
    assert_eq!(
        answer.text,
        format!(
            "workbench_cleanup deleted the folder Scripts/WorkbenchGame/ReforgerBridge with the \
             Handler script from the mod BridgeTest in {}. The mod is ready to pack or publish. \
             Workbench keeps the Handler script loaded until its next script reload. When a \
             Workbench Tool needs the Handler script again, workbench_status copies it back.",
            project.display()
        )
    );
}

#[tokio::test]
async fn deletes_the_empty_folders_above_the_handler_folder() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let project = setup.project("BridgeTest");
    let handler = setup.handler_file();
    fs::create_dir_all(handler.parent().unwrap()).unwrap();
    fs::write(&handler, "class ReforgerBridge {}").unwrap();

    // -- Exec
    let answer = setup.cleanup(json!({ "mod": "bridgetest" })).await;

    // -- Check
    assert!(!answer.is_error, "{}", answer.text);
    assert!(!project.join("Scripts").exists());
    assert!(project.join("addon.gproj").exists());
}

#[tokio::test]
async fn keeps_the_scripts_folder_when_it_holds_another_folder() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let project = setup.project("BridgeTest");
    let handler = setup.handler_file();
    fs::create_dir_all(handler.parent().unwrap()).unwrap();
    fs::write(&handler, "class ReforgerBridge {}").unwrap();
    fs::create_dir_all(project.join("Scripts/Game")).unwrap();

    // -- Exec
    let answer = setup.cleanup(json!({ "mod": "BridgeTest" })).await;

    // -- Check
    assert!(!answer.is_error, "{}", answer.text);
    assert!(!project.join("Scripts/WorkbenchGame").exists());
    assert!(project.join("Scripts/Game").is_dir());
}

#[tokio::test]
async fn deletes_nothing_when_the_mod_has_no_handler_folder() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let project = setup.project("BridgeTest");
    fs::create_dir_all(project.join("Scripts/WorkbenchGame")).unwrap();

    // -- Exec
    let answer = setup.cleanup(json!({ "mod": "BridgeTest" })).await;

    // -- Check
    assert!(!answer.is_error, "{}", answer.text);
    assert_eq!(
        answer.text,
        format!(
            "The mod BridgeTest in {} has no folder Scripts/WorkbenchGame/ReforgerBridge, so \
             workbench_cleanup deleted nothing. The mod is ready to pack or publish.",
            project.display()
        )
    );
    assert!(project.join("Scripts/WorkbenchGame").is_dir());
}

#[tokio::test]
async fn deletes_nothing_on_a_second_call() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let handler = setup.handler_file();
    fs::create_dir_all(handler.parent().unwrap()).unwrap();
    fs::write(&handler, "class ReforgerBridge {}").unwrap();
    let first = setup.cleanup(json!({ "mod": "BridgeTest" })).await;

    // -- Exec
    let second = setup.cleanup(json!({ "mod": "BridgeTest" })).await;

    // -- Check
    assert!(!first.is_error, "{}", first.text);
    assert!(!second.is_error, "{}", second.text);
    assert!(
        second.text.contains("so workbench_cleanup deleted nothing"),
        "{}",
        second.text
    );
}

#[tokio::test]
async fn names_the_real_mods_when_the_mod_is_unknown() {
    // -- Setup & Fixtures
    let setup = Setup::with_projects(&[
        ("Alpha", "4444444444444444", &[]),
        ("Bravo", "5555555555555555", &[]),
    ])
    .with_packed("CoopSlotLobby", "672B195EAD3036D4");

    // -- Exec
    let unknown = setup.cleanup(json!({ "mod": "Charlie" })).await;
    let packed = setup.cleanup(json!({ "mod": "CoopSlotLobby" })).await;

    // -- Check
    assert!(unknown.is_error, "{}", unknown.text);
    assert_eq!(
        unknown.text,
        "The project list of Workbench has no mod \"Charlie\". Its mods: Alpha, Bravo. Call \
         workbench_cleanup again with one of them."
    );
    assert!(packed.is_error, "{}", packed.text);
    assert!(
        packed
            .text
            .starts_with("The project list of Workbench has no mod \"CoopSlotLobby\"."),
        "{}",
        packed.text
    );
}

#[tokio::test]
async fn needs_a_mod_and_deletes_nothing_when_the_mod_is_empty() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let handler = setup.handler_file();
    fs::create_dir_all(handler.parent().unwrap()).unwrap();
    fs::write(&handler, "class ReforgerBridge {}").unwrap();

    // -- Exec
    let empty = setup.cleanup(json!({ "mod": "" })).await;
    let spaces = setup.cleanup(json!({ "mod": "   " })).await;

    // -- Check
    let expected = "workbench_cleanup needs `mod`: the mod to clean up. The project list of \
                    Workbench has these mods: BridgeTest. Call workbench_cleanup again with \
                    `mod` set to one of them.";
    assert!(empty.is_error, "{}", empty.text);
    assert_eq!(empty.text, expected);
    assert!(spaces.is_error, "{}", spaces.text);
    assert_eq!(spaces.text, expected);
    assert!(handler.is_file());
}

#[tokio::test]
async fn needs_a_mod_when_the_mod_is_empty_and_the_project_list_has_no_mods() {
    // -- Setup & Fixtures
    let setup = Setup::with_projects(&[]);

    // -- Exec
    let answer = setup.cleanup(json!({ "mod": "" })).await;

    // -- Check
    assert!(answer.is_error, "{}", answer.text);
    assert_eq!(
        answer.text,
        "workbench_cleanup needs `mod`: the mod to clean up. The project list of Workbench has \
         no mods, so no mod has a Handler script to delete. If the mod is not in the list, call \
         workbench_launch without a mod to open the Workbench launcher, and ask the user to add \
         the mod there (Add Existing). Then call workbench_cleanup again with `mod` set to it."
    );
}

#[tokio::test]
async fn labels_workbench_cleanup_as_destructive_and_safe_to_call_again() {
    // -- Setup & Fixtures
    let setup = Setup::new();
    let settings = setup.settings(Port::Valid(5775), None, Starter::PowerShell);
    let client = test_support::connect(router::<Server>(settings)).await;

    // -- Exec
    let tools = client.list_all_tools().await.unwrap();

    // -- Check
    let tool = tools
        .iter()
        .find(|tool| tool.name == "workbench_cleanup")
        .unwrap();
    let labels = tool.annotations.clone().unwrap();
    assert_eq!(labels.read_only_hint, Some(false));
    assert_eq!(labels.destructive_hint, Some(true));
    assert_eq!(labels.idempotent_hint, Some(true));
    assert_eq!(labels.open_world_hint, Some(false));
}
