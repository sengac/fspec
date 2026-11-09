//! BOARD-022 — Source-shape regression tests.
//!
//! Feature: spec/features/board-search-dialog-with-tab-toggled-id-title-description-modes.feature
//!
//! Pins the BOARD-022 module layout + the RPC-surface invariant: the
//! board search dialog filters the BoardStore snapshot client-side, so
//! NO new `search_work_units` RPC method may appear in the FspecService
//! trait or any FspecBackend transport impl.

use codelet_test_helpers::{OrFail, TestResult};
mod common;

fn workspace_codelet_dir() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    common::workspace_root()
}

fn read_raw(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(codelet_test_helpers::source_composition::read_source(path)
        .or_fail("resolve source shape")?)
}

/// Scenario: the work-unit search dialog component module exists with the documented surface
#[test]
fn work_unit_search_dialog_module_exists_with_documented_surface() -> TestResult {
    // @step Given rust/fspec-tui/src/components/work_unit_search_dialog.rs after BOARD-022 lands
    let path = workspace_codelet_dir()?
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("work_unit_search_dialog.rs");
    // @step Then the file exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path)?;
    // @step And the file contains the dialog id constant "work-unit-search-dialog"
    assert!(
        body.contains("work-unit-search-dialog"),
        "dialog id constant missing"
    );
    // @step And the file contains the SearchMode enum with Id, Title and Description variants
    // BUG-160: the SearchMode enum and the richer-match filter moved to
    // work_unit_search_filter.rs (300-LoC budget); the dialog file
    // re-exports both so the public surface is unchanged.
    let filter = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("work_unit_search_filter.rs"),
    )?;
    assert!(
        filter.contains("enum SearchMode"),
        "SearchMode enum missing from work_unit_search_filter.rs"
    );
    assert!(
        filter.contains("Id"),
        "SearchMode::Id missing from work_unit_search_filter.rs"
    );
    assert!(
        filter.contains("Title"),
        "SearchMode::Title missing from work_unit_search_filter.rs"
    );
    assert!(
        filter.contains("Description"),
        "SearchMode::Description missing from work_unit_search_filter.rs"
    );
    // @step And the file contains the pure filter function "pub fn filter_work_units"
    assert!(
        filter.contains("pub fn filter_work_units"),
        "filter_work_units missing from work_unit_search_filter.rs"
    );
    assert!(
        body.contains("pub use super::work_unit_search_filter"),
        "dialog file must re-export the filter surface (BUG-160)"
    );
    Ok(())
}

/// Scenario: components/mod.rs declares the new module and the two new Action variants
#[test]
fn components_mod_declares_the_new_module_and_action_variants() -> TestResult {
    // @step Given rust/fspec-tui/src/components/mod.rs after BOARD-022 lands
    let body = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("mod.rs"),
    )?;
    // @step Then the file contains the substring "pub mod work_unit_search_dialog"
    assert!(
        body.contains("pub mod work_unit_search_dialog"),
        "module declaration missing"
    );
    // @step And the file contains the Action variant "OpenWorkUnitSearch"
    assert!(
        body.contains("OpenWorkUnitSearch"),
        "Action::OpenWorkUnitSearch missing"
    );
    // @step And the file contains the Action variant "SelectWorkUnit"
    assert!(
        body.contains("SelectWorkUnit"),
        "Action::SelectWorkUnit missing"
    );
    Ok(())
}

/// Scenario: BoardView handle_event gains the modifier-free '/' arm
/// (TUI-129: the board's shortcut keys are the menu registry's keys, so the
/// '/' row of components/menu_bar/items.rs is the board's '/' key).
#[test]
fn board_view_handle_event_gains_the_slash_arm() -> TestResult {
    // @step Given rust/fspec-tui/src/components/menu_bar/items.rs after TUI-129
    let body = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("menu_bar")
            .join("items.rs"),
    )?;
    // @step Then the file contains the substring "key: \"/\""
    assert!(
        body.contains("key: \"/\""),
        "'/' row missing from the menu registry"
    );
    // @step And the file contains the substring "OpenWorkUnitSearch"
    assert!(
        body.contains("OpenWorkUnitSearch"),
        "'/' row must open the work-unit search"
    );
    Ok(())
}

/// Scenario: the board header chord gains the '/ Search' segment
/// (BOARD-023 R10 + MENU-002 R1 + MENU-005 R5 supersession: the header
/// chord row no longer exists — row 3 hosts the live 2-zone menu bar,
/// and the '/ Search' entry now lives in the MenuCategories registry
/// that the bar, the dropdown, the 'u' help dialog and the bare-key
/// docs all read. This pin therefore checks the registry entry instead
/// of the deleted CHORD_HINT chord.)
#[test]
fn board_header_chord_gains_the_search_segment() -> TestResult {
    // @step Given a board with any selection state
    let items = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("menu_bar")
            .join("items.rs"),
    )?;
    // @step When the board is rendered
    // @step Then the header chord row contains the segment "/ Search"
    // (superseded rendering: the '/ Search' entry must be a registry
    // entry with key '/', label "Search" — the single source of truth.)
    assert!(
        items.contains("key: \"/\"") && items.contains("label: \"Search\""),
        "the MenuCategories registry must carry the '/ Search' entry"
    );
    assert!(
        !items.contains("CHORD_HINT"),
        "the registry file must not reference the deleted CHORD_HINT chord"
    );
    Ok(())
}

/// Scenario: the board help content gains the '/' search row
#[test]
fn board_help_content_gains_the_search_row() -> TestResult {
    // @step Given rust/fspec-tui/src/components/menu_bar/items.rs after TUI-129 (the board help lists its rows)
    let body = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("menu_bar")
            .join("items.rs"),
    )?;
    // @step Then the file contains the substring "Search work units"
    assert!(
        body.contains("Search work units"),
        "board help '/' row missing"
    );
    Ok(())
}

/// Scenario: BoardStore gains the work_units/find accessors and the select_work_unit helper
#[test]
fn board_store_gains_the_search_accessors() -> TestResult {
    // @step Given rust/fspec-tui/src/store/board.rs after BOARD-022 lands
    let board = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("store")
            .join("board.rs"),
    )?;
    // @step Then the file contains the substring "pub fn work_units"
    assert!(
        board.contains("pub fn work_units"),
        "work_units accessor missing"
    );
    // @step And the file contains the substring "pub fn find"
    assert!(board.contains("pub fn find"), "find accessor missing");

    // @step Given rust/fspec-tui/src/store/board_viewport.rs after BOARD-022 lands
    let viewport = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("store")
            .join("board_viewport.rs"),
    )?;
    // @step Then the file contains the substring "pub fn select_work_unit"
    assert!(
        viewport.contains("pub fn select_work_unit"),
        "select_work_unit helper missing"
    );
    Ok(())
}

/// Scenario: NO new search_work_units RPC method appears in the service or transports
#[test]
fn no_new_search_work_units_rpc_method_appears() -> TestResult {
    // @step Given the RPC service and the two FspecBackend transport impls after BOARD-022 lands
    let service = read_raw(
        &workspace_codelet_dir()?
            .join("rpc")
            .join("src")
            .join("lib.rs"),
    )?;
    let backend = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("mod.rs"),
    )?;
    let embedded = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("embedded.rs"),
    )?;
    let websocket = read_raw(
        &workspace_codelet_dir()?
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("websocket.rs"),
    )?;
    // @step Then none of the four files contains the substring "search_work_units"
    for (name, body) in [
        ("rpc/src/lib.rs", &service),
        ("transport/mod.rs", &backend),
        ("transport/embedded.rs", &embedded),
        ("transport/websocket.rs", &websocket),
    ] {
        assert!(
            !body.contains("search_work_units"),
            "{name} must NOT gain a search_work_units RPC method (client-side filtering)"
        );
    }
    Ok(())
}
