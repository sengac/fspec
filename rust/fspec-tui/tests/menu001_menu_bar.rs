//! MENU-001 — MenuBar component: 2-zone bar (GUI menu items + anchored
//! dropdown) with view/chip switcher, MenuCategories registry, pure
//! painter.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: pure snapshot structs + stateless painters rendered into
//! `ratatui::buffer::Buffer`s (the `TestBackend` unit-test pattern) —
//! the component holds NO state; the owning surface does.

use codelet_fspec_tui::components::menu_bar::chips::{build_chips, index_prefix, ChipInput};
use codelet_fspec_tui::components::menu_bar::dropdown::{
    dropdown_rect, row_content_width, scroll_window, visible_entry_rows,
};
use codelet_fspec_tui::components::menu_bar::dropdown_paint::render_menu_dropdown;
use codelet_fspec_tui::components::menu_bar::dropdown_text::{key, truncate};
use codelet_fspec_tui::components::menu_bar::items::{
    MenuAction, CATEGORIES, HELP, KANBAN, SETTINGS, TOOLS,
};
use codelet_fspec_tui::components::menu_bar::layout::{menu_bar_layout, DisplayCell, MenuLayout};
use codelet_fspec_tui::components::menu_bar::paint::{paint_menu_bar, MENU_BAR_BG};
use codelet_fspec_tui::components::menu_bar::{MenuChip, MenuFocus, MenuSnapshot, ZoneBCell};
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::Theme;
use codelet_rpc_types::SessionStatus;
use codelet_test_helpers::{OrFail, TestResult};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

/// A chip in `index` (1-based, of 3) with the given status + WU suffix.
fn status_chip(
    index: usize,
    status: SessionStatus,
    wu: Option<&str>,
    clock_ms: u64,
) -> Result<MenuChip, codelet_test_helpers::TestError> {
    Ok(build_chips(
        &[ChipInput {
            index: (index, 3),
            status: Some(status),
            wu_id: wu.map(str::to_string),
            active: false,
            bot: false,
        }],
        clock_ms,
    )
    .pop()
    .or_fail("one chip for one open session")?)
}

/// A chip with the dimmed Idle dot (the fixture default for most rows).
fn chip(i: usize, wu: Option<&'static str>) -> Result<MenuChip, codelet_test_helpers::TestError> {
    status_chip(i + 1, SessionStatus::Idle, wu, 0)
}

/// A snapshot whose Zone B is chips `0..chips.len()` in order.
fn snap(chips: Vec<MenuChip>) -> MenuSnapshot {
    let zone_b = (0..chips.len()).map(ZoneBCell::Chip).collect();
    MenuSnapshot {
        zone_b,
        chips,
        ..Default::default()
    }
}

/// Paint `snap` into a 1-row `w`-column buffer and return (buffer, layout).
fn render(
    snap: &MenuSnapshot,
    w: u16,
) -> Result<(Buffer, MenuLayout), codelet_test_helpers::TestError> {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, snap, &Theme::default())
        .or_fail("layout for non-zero area")?;
    Ok((buf, layout))
}

/// The trimmed row-0 string of a `w`-column buffer.
fn row(buf: &Buffer, w: u16) -> String {
    (0..w)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Paint `snap` into a 1-row `w`-column buffer (layout not needed).
fn paint_only(snap: &MenuSnapshot, w: u16) -> Buffer {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    let _ = paint_menu_bar(area, &mut buf, snap, &Theme::default());
    buf
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Kanban category lists the work-unit workflow entries in order
// (MENU-008 superseded the old "Actions" 7-entry scenario — the registry
// now groups the shortcuts into Kanban / Tools / Settings / Help.)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_registry_pins_the_four_categories_with_their_entry_counts() -> TestResult {
    // @step Given the MenuCategories registry
    // @step When the Zone A category list is read
    assert_eq!(CATEGORIES.len(), 4, "MENU-008: four Zone A categories");
    assert_eq!(CATEGORIES[0].label, "Kanban");
    assert_eq!(CATEGORIES[1].label, "Tools");
    assert_eq!(CATEGORIES[2].label, "Settings");
    assert_eq!(CATEGORIES[3].label, "Help");
    // @step Then it has 5 entries in order: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments", "b New Bot"
    let kanban_keys: Vec<&str> = KANBAN.iter().map(|e| e.key).collect();
    assert_eq!(kanban_keys, vec![".", "/", "D", "A", "b"]);
    assert_eq!(TOOLS.len(), 2);
    assert_eq!(SETTINGS.len(), 4);
    assert_eq!(HELP.len(), 2);
    // @step And each entry's action matches the bare board key (OpenAgentView, OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker)
    assert_eq!(KANBAN[0].action, MenuAction::NewAgent);
    assert_eq!(KANBAN[1].action, MenuAction::Search);
    assert_eq!(KANBAN[2].action, MenuAction::Foundation);
    assert_eq!(KANBAN[3].action, MenuAction::Attachments);
    assert_eq!(KANBAN[4].action, MenuAction::NewBot);
    assert_eq!(TOOLS[0].action, MenuAction::ChangedFiles);
    assert_eq!(TOOLS[1].action, MenuAction::Checkpoints);
    assert_eq!(SETTINGS[0].action, MenuAction::Mux);
    assert_eq!(SETTINGS[1].action, MenuAction::Providers);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Help category lists the help and exit entries
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_help_category_lists_the_help_and_close_entries() -> TestResult {
    // @step Given the MenuCategories registry
    // @step When the Help category is read
    let help = &CATEGORIES[3];
    assert_eq!(help.label, "Help");
    // @step Then it has 2 entries in order: "? Help" and "Esc Close"
    assert_eq!(HELP.len(), 2);
    assert_eq!(HELP[0].key, "?");
    assert_eq!(HELP[0].label, "Help");
    assert_eq!(HELP[1].key, "Esc");
    assert_eq!(HELP[1].label, "Close");
    // @step And they emit OpenBoardHelp and MenuDismissBar (TUI-135: the Close row only closes the menu)
    assert_eq!(HELP[0].action, MenuAction::Help);
    assert_eq!(HELP[1].action, MenuAction::CloseMenu);
    assert!(matches!(
        MenuAction::Help.to_action(),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::CloseMenu.to_action(),
        Action::MenuDismissBar
    ));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Every registry entry has a key hint, label, description and a valid action
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn every_registry_entry_has_a_key_hint_label_description_and_a_valid_action() -> TestResult {
    // @step Given the MenuCategories registry
    // @step When every entry of every category is inspected
    for category in CATEGORIES {
        for entry in category.entries {
            // @step Then none of its key hints, labels or descriptions are empty
            assert!(!entry.key.is_empty(), "key of {:?}", entry.label);
            assert!(!entry.label.is_empty());
            assert!(!entry.description.is_empty());
        }
    }
    // @step And every action resolves to an existing Action variant
    // BUG-199 / BUG-203: the registry mapping is payload-free — the
    // 'New Agent' row ALWAYS mounts the CreateSessionDialog (the
    // shared RPC-060 helper), never a session resume.
    assert!(matches!(
        MenuAction::NewAgent.to_action(),
        Action::OpenCreateSessionDialog { preselect: None }
    ));
    assert!(matches!(
        MenuAction::Search.to_action(),
        Action::OpenWorkUnitSearch
    ));
    assert!(matches!(
        MenuAction::Attachments.to_action(),
        Action::OpenAttachmentPicker
    ));
    assert!(matches!(
        MenuAction::Checkpoints.to_action(),
        Action::OpenCheckpointsView
    ));
    assert!(matches!(
        MenuAction::ChangedFiles.to_action(),
        Action::OpenChangedFilesView
    ));
    assert!(matches!(
        MenuAction::Foundation.to_action(),
        Action::OpenFoundation
    ));
    assert!(matches!(
        MenuAction::Mux.to_action(),
        Action::OpenMuxConfigDialog
    ));
    assert!(matches!(
        MenuAction::Providers.to_action(),
        Action::OpenProviderSettingsView
    ));
    assert!(matches!(
        MenuAction::Help.to_action(),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::Exit.to_action(),
        Action::OpenBoardExitConfirmation
    ));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Three sessions paint chips with per-status glyphs
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn three_sessions_paint_chips_with_per_status_glyphs() -> TestResult {
    // @step Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
    let mut snap = snap(vec![
        status_chip(1, SessionStatus::Idle, None, 0)?,
        status_chip(2, SessionStatus::Running, None, 0)?,
        status_chip(3, SessionStatus::Idle, None, 0)?,
    ]);
    snap.clock_ms = 0;
    // @step When the bar is rendered into a 120-column row
    let (buf, layout) = render(&snap, 120)?;
    let line = row(&buf, 120);
    // @step Then Zone B reads "#1 ●  #2 ⠋  #3 ●" (MENU-011: bracketed)
    assert!(
        line.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"),
        "Zone A first, after the 1-cell R1 pad (MENU-011: bracketed): {line}"
    );
    assert!(line.contains("│"), "separator present: {line}");
    assert!(line.contains("[ #1 ● ]"), "chip 1: {line}");
    assert!(line.contains("[ #2 ⠋ ]"), "chip 2 (Running, 0ms): {line}");
    assert!(line.contains("[ #3 ● ]"), "chip 3: {line}");
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.cell_rects.len(), 3);
    // @step And the Running chip's glyph cell is styled magenta
    let chip2_rect = &layout.cell_rects[1];
    // MENU-011 R2: the glyph sits THREE cells before the rect's end
    // (the " ]" bracket + its leading space: "[ #n … ]").
    let glyph_x = chip2_rect.x + chip2_rect.width - 3;
    assert_eq!(
        buf[(glyph_x, 0)].fg,
        Color::Magenta,
        "Running glyph cell is magenta"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Each session status maps to its own glyph and color
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn each_session_status_maps_to_its_own_glyph_and_color() -> TestResult {
    // @step Given a MenuSnapshot with sessions in statuses Compacting, Paused and Interrupted
    let mut snap = snap(vec![
        status_chip(1, SessionStatus::Compacting, None, 0)?,
        status_chip(2, SessionStatus::Paused, None, 0)?,
        status_chip(3, SessionStatus::Interrupted, None, 0)?,
    ]);
    snap.clock_ms = 0;
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 120)?;
    // @step Then the chips render "↻", "!" and "✕" (MENU-011: the
    // glyph sits three cells inside the right bracket: "[ … ]")
    for (i, glyph) in ["↻", "!", "✕"].iter().enumerate() {
        let rect = &layout.cell_rects[i];
        assert_eq!(
            buf[(rect.x + rect.width - 3, 0)].symbol(),
            *glyph,
            "chip {i} glyph"
        );
    }
    // @step And the glyphs are styled cyan, yellow and red respectively
    let colors = [Color::Cyan, Color::Yellow, Color::Red];
    for (i, color) in colors.iter().enumerate() {
        let rect = &layout.cell_rects[i];
        assert_eq!(
            buf[(rect.x + rect.width - 3, 0)].fg,
            *color,
            "chip {i} glyph color"
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A single open session shows the #1 prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_single_open_session_shows_the_1_prefix() -> TestResult {
    // @step Given a MenuSnapshot with exactly 1 open session (Idle)
    let snap = snap(vec![status_chip(1, SessionStatus::Idle, None, 0)?]);
    // @step When the bar is rendered
    let (buf, _) = render(&snap, 80)?;
    // @step Then Zone B reads "#1 ●" (the prefix paints because n >= 1, SessionHeader parity)
    assert!(row(&buf, 80).contains("#1 ●"));
    assert_eq!(index_prefix((1, 1)), "#1 ");
    assert_eq!(index_prefix((0, 0)), "", "n == 0 paints no prefix");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Running chip's braille glyph advances with the clock
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_running_chips_braille_glyph_advances_with_the_clock() -> TestResult {
    // @step Given a MenuSnapshot with 1 open session (Running)
    // @step When the bar is rendered at clock_ms 0 and again at clock_ms 80
    let at_0 = status_chip(1, SessionStatus::Running, None, 0)?;
    let at_80 = status_chip(1, SessionStatus::Running, None, 80)?;
    // @step Then the chip's glyph is the first braille frame at 0ms and the second frame at 80ms
    assert_eq!(at_0.glyph, "⠋", "frame 0");
    assert_eq!(at_80.glyph, "⠙", "frame 1 (80 ms cadence)");
    let at_240 = status_chip(1, SessionStatus::Running, None, 240)?;
    assert_eq!(at_240.glyph, "⠸", "frame 3");
    assert_eq!(at_0.glyph_style.fg, Some(Color::Magenta));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cleared sessions produce no chip
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cleared_sessions_produce_no_chip() -> TestResult {
    // @step Given a MenuSnapshot whose open session list contains a session in the Cleared status
    let inputs = [
        ChipInput {
            index: (1, 2),
            status: Some(SessionStatus::Cleared),
            wu_id: None,
            active: false,
            bot: false,
        },
        ChipInput {
            index: (2, 2),
            status: Some(SessionStatus::Idle),
            wu_id: None,
            active: false,
            bot: false,
        },
    ];
    // @step When the chip list is built
    let chips = build_chips(&inputs, 0);
    // @step Then no chip is produced for the Cleared session
    assert_eq!(chips.len(), 1, "the Cleared session is dropped");
    assert_eq!(chips[0].index, (2, 2), "only the open session remains");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A chip shows its work-unit id when width allows
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_chip_shows_its_work_unit_id_when_width_allows() -> TestResult {
    // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    let snap = snap(vec![status_chip(
        1,
        SessionStatus::Idle,
        Some("MENU-001"),
        0,
    )?]);
    // @step And a 200-column render area
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 200)?;
    // @step Then the chip reads "#1 MENU-001 ●"
    assert_eq!(layout.level, 0, "wide area keeps the WU suffix");
    assert!(
        row(&buf, 200).contains("#1 MENU-001 ●"),
        "chip with WU id: {}",
        row(&buf, 200)
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The focused menu item paints inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_focused_menu_item_paints_inverse_video() -> TestResult {
    // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
    let mut snap = snap(vec![chip(0, None)?]);
    snap.focus = Some(MenuFocus::Item(0));
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80)?;
    // @step Then the "Kanban" item's cells are styled bg Cyan fg Black
    let kanban_rect = &layout.item_rects[0];
    for x in kanban_rect.x..kanban_rect.x + kanban_rect.width {
        assert_eq!(buf[(x, 0)].bg, Color::Cyan, "Kanban cell {x}");
        assert_eq!(buf[(x, 0)].fg, Color::Black, "Kanban cell {x}");
        assert!(
            buf[(x, 0)].modifier.contains(Modifier::BOLD),
            "Kanban cell {x} bold"
        );
    }
    // @step And the "Help" item's cells are not inverted
    let help_rect = &layout.item_rects[3];
    for x in help_rect.x..help_rect.x + help_rect.width {
        assert_ne!(buf[(x, 0)].bg, Color::Cyan, "Help cell {x}");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The focused chip paints inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_focused_chip_paints_inverse_video() -> TestResult {
    // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
    let mut snap = snap(vec![chip(0, None)?, chip(1, None)?]);
    snap.focus = Some(MenuFocus::ZoneB(1));
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80)?;
    // @step Then the second chip's cells are styled bg Cyan fg Black
    let rect = &layout.cell_rects[1];
    assert_eq!(
        buf[(rect.x, 0)].bg,
        Color::Cyan,
        "focused chip cell inverted"
    );
    assert_eq!(
        buf[(rect.x + rect.width - 1, 0)].bg,
        Color::Cyan,
        "whole chip cell inverted"
    );
    // @step And the first chip's cells are not inverted
    let rect0 = &layout.cell_rects[0];
    assert_ne!(buf[(rect0.x, 0)].bg, Color::Cyan);
    assert_ne!(buf[(rect0.x + rect0.width - 1, 0)].bg, Color::Cyan);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: No focus paints no highlight
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn no_focus_paints_no_highlight() -> TestResult {
    // @step Given a MenuSnapshot with MenuFocus None
    let snap = snap(vec![chip(0, None)?, chip(1, None)?]);
    // @step When the bar is rendered
    let buf = paint_only(&snap, 80);
    // @step Then no cell in the row is styled bg Cyan
    for x in 0..80u16 {
        assert_ne!(
            buf[(x, 0)].bg,
            Color::Cyan,
            "cell {x} unexpectedly inverse without a focus"
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The row paints 1-cell padding and a dark background on every cell
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_row_paints_1_cell_padding_and_a_dark_background_on_every_cell() -> TestResult {
    // @step Given a 50-column render area
    // (MENU-011: the bracketed Zone A is 42 cells wide — a 40-col area
    // no longer fits even the absolute minimum; 50 keeps the layout.)
    // @step When the bar is rendered
    let snap = snap(vec![chip(0, None)?]);
    let (buf, _) = render(&snap, 50)?;
    // @step Then cell 0 and the last cell are background-only
    assert_eq!(buf[(0, 0)].symbol(), " ", "cell 0 is the pad");
    assert_eq!(buf[(49, 0)].symbol(), " ", "last cell is the pad");
    // @step And every cell of the row has the #333333 background
    for x in 0..50u16 {
        assert_eq!(buf[(x, 0)].bg, MENU_BAR_BG, "cell {x} missing bg");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An empty chip list omits the separator and Zone B
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn an_empty_chip_list_omits_the_separator_and_zone_b() -> TestResult {
    // @step Given a MenuSnapshot with no open sessions
    let snap = MenuSnapshot::default();
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80)?;
    // @step Then the row contains the menu items but no "|" separator and no chips
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    let line = row(&buf, 80);
    assert!(!line.contains("│"));
    assert!(line.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"));
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Tight width drops WU ids before folding chips
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tight_width_drops_wu_ids_before_folding_chips() -> TestResult {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let snap = snap(vec![
        status_chip(1, SessionStatus::Idle, Some("MENU-001-LONGID"), 0)?,
        status_chip(2, SessionStatus::Idle, Some("MENU-002-LONGID"), 0)?,
        status_chip(3, SessionStatus::Idle, Some("MENU-003-LONGID"), 0)?,
        status_chip(4, SessionStatus::Idle, Some("MENU-004-LONGID"), 0)?,
    ]);
    // @step When the bar is rendered into a 88-column area
    // (MENU-008: Zone A is now 26 cells wide — "Kanban Tools Settings
    // Help"; MENU-011: the BRACKETED Zone A is 42 cells — so the
    // old 52-col fixture would truncate further than intended; 88 keeps
    // level 1: WU ids dropped, every chip still paints.)
    let (buf, layout) = render(&snap, 88)?;
    // @step Then no chip shows a work-unit id suffix
    assert!(
        layout.level == 1,
        "WU ids dropped first (got level {})",
        layout.level
    );
    let line = row(&buf, 88);
    assert!(!line.contains("MENU-001-LONGID"));
    assert!(!line.contains("MENU-004-LONGID"));
    // @step And the chips still paint (MENU-011: bracketed, none folded)
    assert!(line.contains("[ #1 ● ]"), "chip 1: {line}");
    assert!(line.contains("[ #4 ● ]"), "chip 4: {line}");
    let folds = layout
        .cells
        .iter()
        .filter(|c| matches!(c, DisplayCell::Fold { .. }))
        .count();
    assert_eq!(folds, 0, "no fold marker at this width");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Extremely tight width degrades to the absolute minimum
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn extremely_tight_width_degrades_to_the_absolute_minimum() -> TestResult {
    // @step Given a MenuSnapshot with 6 open sessions
    let snap = snap(vec![
        status_chip(1, SessionStatus::Idle, Some("MENU-001"), 0)?,
        status_chip(2, SessionStatus::Idle, Some("MENU-002"), 0)?,
        status_chip(3, SessionStatus::Idle, Some("MENU-003"), 0)?,
        status_chip(4, SessionStatus::Idle, Some("MENU-004"), 0)?,
        status_chip(5, SessionStatus::Idle, Some("MENU-005"), 0)?,
        status_chip(6, SessionStatus::Idle, Some("MENU-006"), 0)?,
    ]);
    // @step When the bar is rendered into a 44-column area
    // (MENU-011: the bracketed Zone A is 42 cells; with the 1-cell padding
    // each side, 44 columns is the narrowest area that still paints.)
    let layout = menu_bar_layout(Rect::new(0, 0, 44, 1), &snap)
        .or_fail("44 columns still affords the bracketed Zone A")?;
    // @step Then the row still fits within the area width
    let last = layout
        .cell_rects
        .last()
        .or_else(|| layout.item_rects.last());
    assert!(
        last.is_some_and(|r| r.x + r.width <= 44),
        "rightmost painted cell stays inside the area"
    );
    // @step And it shows only the Zone A items "[ Kanban ] [ Tools ] [ Settings ] [ Help ]" (MENU-011: the bracketed Zone A is the absolute minimum; no chips)
    assert_eq!(layout.level, 4, "every Zone B cell is dropped");
    assert!(
        layout.cell_rects.is_empty(),
        "no chip paints at the minimum"
    );
    assert!(
        layout.separator.is_none(),
        "no zone separator at the minimum"
    );
    let line = row(&paint_only(&snap, 44), 44);
    assert_eq!(
        line.trim_end(),
        " [ Kanban ] [ Tools ] [ Settings ] [ Help ]",
        "only Zone A paints: {line}"
    );
    // One column fewer and even Zone A no longer fits: nothing paints.
    assert!(menu_bar_layout(Rect::new(0, 0, 43, 1), &snap).is_none());
    Ok(())
}

proptest::proptest! {
    #[test]
    fn painted_row_never_exceeds_the_area_width(
        width in 3u16..=400,
        chip_count in 0usize..=24,
    ) {
        let chips: Vec<MenuChip> = (0..chip_count)
            .map(|i| status_chip(i + 1, SessionStatus::Idle, Some("A-BIG-UNIT-ID"), 0))
            .collect::<Result<_, _>>()
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let snap = snap(chips);
        // @step Then the row still fits within the area width
        if let Some(layout) = menu_bar_layout(Rect::new(0, 0, width, 1), &snap) {
            // Too-tight areas legitimately paint nothing — only
            // assert the bound when a layout was produced.
            if let Some(last) = layout.cell_rects.last() {
                assert!(
                    last.x + last.width <= width,
                    "last cell overflows: x {} w {} at width {}",
                    last.x,
                    last.width,
                    width
                );
            }
            if let Some(last) = layout.item_rects.last() {
                assert!(
                    last.x + last.width <= width,
                    "last item overflows: x {} w {} at width {}",
                    last.x,
                    last.width,
                    width
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A zero-width or zero-height area paints nothing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_zero_width_or_zero_height_area_paints_nothing() -> TestResult {
    // @step Given a render area with width 0 or height 0
    let snap = snap(vec![chip(0, None)?]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    // @step When the bar is rendered
    // @step Then the buffer is unchanged
    assert!(paint_menu_bar(Rect::new(0, 0, 0, 1), &mut buf, &snap, &Theme::default()).is_none());
    assert!(paint_menu_bar(Rect::new(0, 0, 10, 0), &mut buf, &snap, &Theme::default()).is_none());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown panel is anchored under its menu item
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_panel_is_anchored_under_its_menu_item() -> TestResult {
    // @step Given a MenuSnapshot with the Kanban dropdown open at cursor 1
    // (Kanban = category 0; the item x is the 1-cell R1 pad in a
    // standard 80x24 bar)
    let item_x: u16 = 1;
    // @step When the dropdown is rendered
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), item_x, 0, 0).or_fail("panel")?;
    // @step Then the panel's x equals the Kanban item's x and its y is one row below the bar
    assert_eq!(panel.x, item_x);
    assert_eq!(panel.y, 1, "one row below the bar");
    // @step And its height is 7 rows (5 entries + 2 border rows)
    assert_eq!(panel.height, 7, "5 entries + 2 border rows");
    // @step And it has no title row and no footer row
    // (the height is exactly entries + 2; the width is
    // max(24, widest_row + 4) with the widest Kanban row at 48)
    assert_eq!(panel.width, 52, "max(24, 48 + 4) — no title/footer padding");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dropdown rows show a key-hint column, label and dim description
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dropdown_rows_show_a_key_hint_column_label_and_dim_description() -> TestResult {
    // @step Given the Actions dropdown rendered open
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 1, 0, 0).or_fail("panel")?;
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    // @step When row 2 is inspected
    let y2 = panel.y + 1 + 1;
    // @step Then it starts with the dim 4-cell key hint "/ "
    assert_eq!(buf[(panel.x + 1, y2)].symbol(), "/");
    assert_eq!(
        buf[(panel.x + 1 + 3, y2)].symbol(),
        " ",
        "key column pads to 4 cells"
    );
    // @step And it shows the label "Search" in the primary foreground
    let theme = Theme::default();
    assert_eq!(
        buf[(panel.x + 1 + 4, y2)].symbol(),
        "S",
        "label starts at offset 4"
    );
    assert_eq!(buf[(panel.x + 1 + 4, y2)].fg, theme.fg);
    assert_eq!(buf[(panel.x + 1, y2)].fg, theme.dim, "key hint is dim");
    // @step And it shows a dim description truncated with an ellipsis when the row is too wide
    assert_eq!(
        buf[(panel.x + 1, panel.y + 1)].symbol(),
        ".",
        "row 1 (index 0) keeps its key hint"
    );
    let desc = truncate("Open the attachment picker for the selected work unit", 10);
    assert!(desc.ends_with('…'), "long descriptions truncate: {desc}");
    assert_eq!(desc.width(), 9, "truncation stays within the budget");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown cursor row is inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_cursor_row_is_inverse_video() -> TestResult {
    // @step Given the Actions dropdown open with the cursor on row 1
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 1, 0, 0).or_fail("panel")?;
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 1, &Theme::default());
    // @step When the dropdown is rendered
    let y = panel.y + 1 + 1;
    // @step Then row 2 (index 1) is styled bg Cyan fg Black bold across its full inner width
    let inner_end = panel.right().saturating_sub(2);
    for x in panel.x + 1..=inner_end {
        assert_eq!(buf[(x, y)].bg, Color::Cyan, "inner cell {x} inverted");
    }
    let label_x = panel.x + 1 + 4;
    assert_eq!(buf[(label_x, y)].fg, Color::Black);
    assert!(buf[(label_x, y)].modifier.contains(Modifier::BOLD));
    // @step And all other rows are not inverted
    let other_y = panel.y + 1 + 3;
    assert_ne!(buf[(panel.x + 1, other_y)].bg, Color::Cyan);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown left-clamps at the terminal right edge
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_left_clamps_at_the_terminal_right_edge() -> TestResult {
    // @step Given a Help item positioned near the terminal's right edge
    // @step When its dropdown is rendered
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 40, 0, 0).or_fail("panel")?;
    // @step Then the panel is shifted left so it never paints past the terminal width
    assert!(
        panel.x + panel.width <= 80,
        "panel stays inside: x {} w {}",
        panel.x,
        panel.width
    );
    assert!(panel.x < 40, "shifted left from the item");
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown bottom-clips with an ellipsis when the terminal is too short
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_bottom_clips_with_an_ellipsis_when_the_terminal_is_too_short() -> TestResult {
    // @step Given a short render area (6 rows below the bar)
    let panel = dropdown_rect(Rect::new(0, 0, 80, 7), 1, 0, 0).or_fail("panel")?;
    assert_eq!(panel.height, 6);
    // @step When the Kanban dropdown (5 entries) is rendered
    let entries = CATEGORIES[0].entries.len();
    assert_eq!(entries, 5, "Kanban has 5 entries (TUI-114 added New Bot)");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    // @step Then only 3 entry rows plus a dim "⋯" indicator row are visible (4 content rows inside the borders)
    assert_eq!(visible_entry_rows(panel, entries), 3, "3 entry rows");
    let ellipsis_y = panel.y + 1 + 3;
    assert_eq!(buf[(panel.x + 1, ellipsis_y)].symbol(), "⋯");
    // @step And the panel never paints below the area
    assert!(panel.bottom() <= 7);
    Ok(())
}

#[test]
fn the_dropdown_cursor_never_clips_out_of_a_bottom_clipped_panel() -> TestResult {
    // 5 rows below the bar: content = 3 rows → 2 entry rows + ⋯.
    let panel = dropdown_rect(Rect::new(0, 0, 80, 6), 1, 0, 0).or_fail("panel")?;
    assert_eq!(visible_entry_rows(panel, 4), 2, "2 entry rows");
    // Cursor on the last entry → the window scrolls to 2..=3.
    let start = scroll_window(panel, 4, 3);
    assert_eq!(start, 2, "rows 2..3 visible, cursor row last");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 3, &Theme::default());
    // The 'A' key of the last entry ('A Attachments') is painted on
    // content row 1 (the last visible entry row).
    let y = panel.y + 1 + 1;
    assert_eq!(buf[(panel.x + 1, y)].symbol(), "A");
    // And the ellipsis sits on the final content row.
    assert_eq!(buf[(panel.x + 1, panel.y + 1 + 2)].symbol(), "⋯");
    Ok(())
}

#[test]
fn no_space_below_the_bar_yields_no_panel() -> TestResult {
    assert!(dropdown_rect(Rect::new(0, 0, 80, 1), 1, 0, 0).is_none());
    assert!(dropdown_rect(Rect::new(0, 0, 10, 24), 1, 0, 0).is_none());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Supporting ring-math + registry-pin tests (R3, R5, R6 geometry helpers)
// ─────────────────────────────────────────────────────────────────────────

/// 2 items + 3 chips, all in Zone B (non-mux shape).
fn ring_snap3() -> Result<MenuSnapshot, codelet_test_helpers::TestError> {
    Ok(snap(vec![chip(0, None)?, chip(1, None)?, chip(2, None)?]))
}

#[test]
fn ring_walks_items_then_zone_b_then_wraps_forward() -> TestResult {
    let s = ring_snap3()?;
    assert_eq!(
        s.advance(None, 1),
        MenuFocus::Item(0),
        "no focus + right starts at the first item"
    );
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
    assert_eq!(
        s.advance(Some(MenuFocus::Item(3)), 1),
        MenuFocus::ZoneB(0),
        "running out of items lands on the first Zone B cell"
    );
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(2)), 1), MenuFocus::Item(0));
    Ok(())
}

#[test]
fn ring_walks_backwards_with_wrap() -> TestResult {
    let s = ring_snap3()?;
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), -1), MenuFocus::ZoneB(2));
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), -1), MenuFocus::Item(3));
    Ok(())
}

#[test]
fn ring_follows_the_display_order_in_mux_mode() -> TestResult {
    // Mux: Zone B = Board view, chip 1, chip 0, chip 2 (pane order).
    let chips = vec![chip(0, None)?, chip(1, None)?, chip(2, None)?];
    let s = MenuSnapshot {
        zone_b: vec![
            ZoneBCell::View {
                label: "Board",
                active: true,
            },
            ZoneBCell::Chip(1),
            ZoneBCell::Chip(0),
            ZoneBCell::Chip(2),
        ],
        chips,
        ..Default::default()
    };
    assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::ZoneB(0));
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), 1), MenuFocus::ZoneB(1));
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(3)), 1), MenuFocus::Item(0));
    Ok(())
}

#[test]
fn empty_zone_b_ring_wraps_items_onto_themselves() -> TestResult {
    let s = MenuSnapshot::default();
    assert_eq!(s.advance(None, 1), MenuFocus::Item(0));
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
    assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::Item(0));
    Ok(())
}

#[test]
fn dropdown_cursor_clamps_into_the_category_range() -> TestResult {
    let s = MenuSnapshot::default();
    assert_eq!(s.dropdown_cursor(0, 4), 4, "Kanban has 5 rows");
    assert_eq!(s.dropdown_cursor(0, 5), 0, "wraps past the end");
    assert_eq!(s.dropdown_cursor(3, 1), 1, "Help has 2 rows");
    assert_eq!(s.dropdown_cursor(3, 2), 0, "Help wraps");
    Ok(())
}

#[test]
fn chip_for_cell_resolves_only_chip_cells() -> TestResult {
    let s = ring_snap3()?;
    assert_eq!(s.chip_for_cell(0), Some(0));
    assert_eq!(s.chip_for_cell(2), Some(2));
    assert_eq!(s.chip_for_cell(9), None, "out of range");
    Ok(())
}

#[test]
fn open_dropdown_marks_its_item_active() -> TestResult {
    let mut snap = snap(vec![chip(0, None)?]);
    snap.open_menu = Some((3, 0));
    let (buf, layout) = render(&snap, 80)?;
    let help_x = layout.item_rects[3].x;
    assert_eq!(buf[(help_x, 0)].fg, Color::Black);
    Ok(())
}

#[test]
fn wu_ids_count_toward_width_only_at_level_zero() -> TestResult {
    let snap = snap(vec![status_chip(
        1,
        SessionStatus::Idle,
        Some("MENU-001"),
        0,
    )?]);
    let full = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).or_fail("expected a value")?;
    assert_eq!(full.level, 0);
    assert_eq!(
        full.cell_rects[0].width,
        3 + 8 + 1 + 1 + 4,
        "#1 prefix + WU id + space + glyph + the MENU-011 '[ ]' brackets"
    );
    // 60 cols: the level-0 row (42 bracketed Zone A + 3 separator +
    // 17 chip = 62 inner cells) no longer fits, so the WU id
    // suffix is the first thing dropped (R6 step 1).
    let tight = menu_bar_layout(Rect::new(0, 0, 60, 1), &snap).or_fail("expected a value")?;
    assert_eq!(tight.level, 1, "WU id dropped first");
    Ok(())
}

#[test]
fn full_width_keeps_level_zero_with_every_cell() -> TestResult {
    let snap = snap(vec![chip(0, None)?, chip(1, None)?, chip(2, None)?]);
    let layout =
        menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).or_fail("layout for 120-col area")?;
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.item_rects.len(), 4);
    assert_eq!(layout.cell_rects.len(), 3);
    assert!(layout.separator.is_some());
    Ok(())
}

#[test]
fn very_tight_width_folds_chips_beyond_three() -> TestResult {
    let chips = (0..6).map(|i| chip(i, None)).collect::<Result<_, _>>()?;
    // MENU-011: the bracketed Zone A is 42 cells — a 30-col area no
    // longer fits even the absolute minimum. At 90 cols (88 inner) the
    // level-1 row (61) still fits all 6 chips, so the ladder must fold
    // at level 2: the 4th-6th chips become a single `+N` marker.
    let layout = menu_bar_layout(Rect::new(0, 0, 90, 1), &snap(chips)).or_fail("layout")?;
    assert!(layout.level >= 2, "folding level used: {}", layout.level);
    let fold_count = layout
        .cells
        .iter()
        .filter(|c| matches!(c, DisplayCell::Fold { .. }))
        .count();
    assert_eq!(fold_count, 1, "a single +N marker");
    Ok(())
}

#[test]
fn no_chips_is_the_full_level_with_no_zone_b() -> TestResult {
    // R6: with no chips the separator and Zone B are omitted
    // entirely — no truncation is needed, so the level stays 0.
    let layout = menu_bar_layout(Rect::new(0, 0, 80, 1), &MenuSnapshot::default())
        .or_fail("expected a value")?;
    assert_eq!(layout.level, 0, "no chips → nothing to truncate");
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    assert!(layout.cell_rects.is_empty());
    assert_eq!(layout.item_rects.len(), 4, "Zone A still lays out");
    Ok(())
}

#[test]
fn zero_or_tiny_area_yields_no_layout() -> TestResult {
    let s = snap(vec![chip(0, None)?]);
    assert!(menu_bar_layout(Rect::new(0, 0, 0, 1), &s).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 10, 0), &s).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 2, 1), &s).is_none());
    Ok(())
}

#[test]
fn key_hints_fit_the_four_cell_column() -> TestResult {
    assert_eq!(key("."), ".   ");
    assert_eq!(key("Esc"), "Esc ");
    Ok(())
}

#[test]
fn truncate_keeps_short_text_and_caps_long_text() -> TestResult {
    assert_eq!(truncate("short", 10), "short");
    assert_eq!(truncate("abc", 0), "");
    assert_eq!(truncate("exactly-10", 10), "exactly-10");
    Ok(())
}

#[test]
fn row_content_width_is_key_col_plus_label_plus_separator_plus_capped_description() -> TestResult {
    // The widest Actions row: 4 (key col) + 13 ("FOUNDATION.md") + 3
    // (" - ") + 28 (capped description) = 48.
    assert_eq!(
        row_content_width("FOUNDATION.md", "Open FOUNDATION.md in the browser"),
        48
    );
    // A short description is not capped ("Search work units" = 17 wide).
    assert_eq!(
        row_content_width("Search", "Search work units"),
        4 + 6 + 3 + 17
    );
    Ok(())
}
