//! MENU-001/MENU-008 — unit tests for the MenuCategories registry.
//!
//! Split from `items.rs` so the registry module stays under the
//! 300-LoC ceiling (MENU-008 R1: four categories).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::SessionId;

use super::{MenuAction, CATEGORIES, HELP, KANBAN, SETTINGS, TOOLS};
use crate::components::Action;

#[test]
fn categories_are_kanban_tools_settings_then_help_with_the_expected_entry_counts() {
    // @step Given the MenuCategories registry
    // @step When the Help category is read
    // @step Then it has 2 entries in order: "? Help" and "Esc Exit"
    assert_eq!(CATEGORIES.len(), 4);
    assert_eq!(CATEGORIES[0].label, "Kanban");
    assert_eq!(CATEGORIES[1].label, "Tools");
    assert_eq!(CATEGORIES[2].label, "Settings");
    assert_eq!(CATEGORIES[3].label, "Help");
    assert_eq!(KANBAN.len(), 4);
    assert_eq!(TOOLS.len(), 2);
    assert_eq!(SETTINGS.len(), 2);
    assert_eq!(HELP.len(), 2);
    // @step And they emit OpenBoardHelp and OpenBoardExitConfirmation
    assert_eq!(HELP[0].action, MenuAction::Help);
    assert_eq!(HELP[1].action, MenuAction::Exit);
    assert_eq!(HELP[0].key, "?");
    assert_eq!(HELP[1].key, "Esc");
}

#[test]
fn kanban_entries_keep_the_legacy_board_workflow_shortcuts() {
    // @step Given the MenuCategories registry
    // @step When the Kanban category is read
    // @step Then it has 4 entries in order: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    let keys: Vec<&str> = KANBAN.iter().map(|e| e.key).collect();
    assert_eq!(
        keys,
        vec![".", "/", "D", "A"],
        "the legacy workflow arms (., /, D, A) in Kanban order"
    );
}

#[test]
fn tools_entries_list_the_git_tools() {
    // @step Given the MenuCategories registry
    // @step When the Tools category is read
    // @step Then it has 2 entries in order: "F Changed Files" and "C Checkpoints"
    let keys: Vec<&str> = TOOLS.iter().map(|e| e.key).collect();
    assert_eq!(keys, vec!["F", "C"], "Changed Files then Checkpoints");
    // @step And they emit OpenChangedFilesView and OpenCheckpointsView
    assert_eq!(TOOLS[0].action, MenuAction::ChangedFiles);
    assert_eq!(TOOLS[1].action, MenuAction::Checkpoints);
}

#[test]
fn settings_entries_list_the_configuration_entries() {
    // @step Given the MenuCategories registry
    // @step When the Settings category is read
    // @step Then it has 2 entries in order: "M Mux" and "P Providers"
    let keys: Vec<&str> = SETTINGS.iter().map(|e| e.key).collect();
    assert_eq!(keys, vec!["M", "P"], "Mux layout then Providers");
    // @step And they emit OpenMuxConfigDialog and OpenProviderSettingsView
    assert_eq!(SETTINGS[0].action, MenuAction::Mux);
    assert_eq!(SETTINGS[1].action, MenuAction::Providers);
}

#[test]
fn every_entry_has_a_non_empty_key_label_and_description() {
    // @step When every entry of every category is inspected
    // @step Then none of its key hints, labels or descriptions are empty
    for category in CATEGORIES {
        for entry in category.entries {
            assert!(!entry.key.is_empty(), "key of {:?}", entry.label);
            assert!(!entry.label.is_empty());
            assert!(!entry.description.is_empty());
        }
    }
}

#[test]
fn every_action_maps_to_an_existing_bus_variant() {
    // @step And each entry's action matches the bare board key (OpenAgentView, OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker)
    // @step And every action resolves to an existing Action variant
    let target = Some(SessionId::new("s-1"));
    assert!(matches!(
        MenuAction::NewAgent.to_action(target),
        Action::OpenAgentView(Some(_))
    ));
    assert!(matches!(
        MenuAction::NewAgent.to_action(None),
        Action::OpenAgentView(None)
    ));
    assert!(matches!(
        MenuAction::Search.to_action(None),
        Action::OpenWorkUnitSearch
    ));
    assert!(matches!(
        MenuAction::Attachments.to_action(None),
        Action::OpenAttachmentPicker
    ));
    assert!(matches!(
        MenuAction::Checkpoints.to_action(None),
        Action::OpenCheckpointsView
    ));
    assert!(matches!(
        MenuAction::ChangedFiles.to_action(None),
        Action::OpenChangedFilesView
    ));
    assert!(matches!(
        MenuAction::Foundation.to_action(None),
        Action::OpenFoundation
    ));
    assert!(matches!(
        MenuAction::Mux.to_action(None),
        Action::OpenMuxConfigDialog
    ));
    assert!(matches!(
        MenuAction::Providers.to_action(None),
        Action::OpenProviderSettingsView
    ));
    assert!(matches!(
        MenuAction::Help.to_action(None),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::Exit.to_action(None),
        Action::OpenBoardExitConfirmation
    ));
}
