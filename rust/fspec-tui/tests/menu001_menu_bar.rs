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

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
use codelet_rpc_types::{SessionId, SessionStatus};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

/// A chip in `index` (1-based, of 3) with the given status + WU suffix.
fn status_chip(index: usize, status: SessionStatus, wu: Option<&str>, clock_ms: u64) -> MenuChip {
    build_chips(
        &[ChipInput {
            index: (index, 3),
            status,
            wu_id: wu.map(str::to_string),
            active: false,
        }],
        clock_ms,
    )
    .pop()
    .expect("one chip for one open session")
}

/// A chip with the dimmed Idle dot (the fixture default for most rows).
fn chip(i: usize, wu: Option<&'static str>) -> MenuChip {
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
fn render(snap: &MenuSnapshot, w: u16) -> (Buffer, MenuLayout) {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    let layout =
        paint_menu_bar(area, &mut buf, snap, &Theme::default()).expect("layout for non-zero area");
    (buf, layout)
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
fn the_registry_pins_the_four_categories_with_their_entry_counts() {
    // @step Given the MenuCategories registry
    // @step When the Zone A category list is read
    assert_eq!(CATEGORIES.len(), 4, "MENU-008: four Zone A categories");
    assert_eq!(CATEGORIES[0].label, "Kanban");
    assert_eq!(CATEGORIES[1].label, "Tools");
    assert_eq!(CATEGORIES[2].label, "Settings");
    assert_eq!(CATEGORIES[3].label, "Help");
    // @step Then it has 4 entries in order: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    let kanban_keys: Vec<&str> = KANBAN.iter().map(|e| e.key).collect();
    assert_eq!(kanban_keys, vec![".", "/", "D", "A"]);
    assert_eq!(TOOLS.len(), 2);
    assert_eq!(SETTINGS.len(), 2);
    assert_eq!(HELP.len(), 2);
    // @step And each entry's action matches the bare board key (OpenAgentView, OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker)
    assert_eq!(KANBAN[0].action, MenuAction::NewAgent);
    assert_eq!(KANBAN[1].action, MenuAction::Search);
    assert_eq!(KANBAN[2].action, MenuAction::Foundation);
    assert_eq!(KANBAN[3].action, MenuAction::Attachments);
    assert_eq!(TOOLS[0].action, MenuAction::ChangedFiles);
    assert_eq!(TOOLS[1].action, MenuAction::Checkpoints);
    assert_eq!(SETTINGS[0].action, MenuAction::Mux);
    assert_eq!(SETTINGS[1].action, MenuAction::Providers);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Help category lists the help and exit entries
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_help_category_lists_the_help_and_exit_entries() {
    // @step Given the MenuCategories registry
    // @step When the Help category is read
    let help = &CATEGORIES[3];
    assert_eq!(help.label, "Help");
    // @step Then it has 2 entries in order: "? Help" and "Esc Exit"
    assert_eq!(HELP.len(), 2);
    assert_eq!(HELP[0].key, "?");
    assert_eq!(HELP[0].label, "Help");
    assert_eq!(HELP[1].key, "Esc");
    assert_eq!(HELP[1].label, "Exit");
    // @step And they emit OpenBoardHelp and OpenBoardExitConfirmation
    assert_eq!(HELP[0].action, MenuAction::Help);
    assert_eq!(HELP[1].action, MenuAction::Exit);
    assert!(matches!(
        MenuAction::Help.to_action(None),
        Action::OpenBoardHelp
    ));
    assert!(matches!(
        MenuAction::Exit.to_action(None),
        Action::OpenBoardExitConfirmation
    ));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Every registry entry has a key hint, label, description and a valid action
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn every_registry_entry_has_a_key_hint_label_description_and_a_valid_action() {
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Three sessions paint chips with per-status glyphs
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn three_sessions_paint_chips_with_per_status_glyphs() {
    // @step Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
    let mut snap = snap(vec![
        status_chip(1, SessionStatus::Idle, None, 0),
        status_chip(2, SessionStatus::Running, None, 0),
        status_chip(3, SessionStatus::Idle, None, 0),
    ]);
    snap.clock_ms = 0;
    // @step When the bar is rendered into a 120-column row
    let (buf, layout) = render(&snap, 120);
    let line = row(&buf, 120);
    // @step Then Zone B reads "#1 ●  #2 ⠋  #3 ●"
    assert!(
        line.starts_with(" Kanban Tools Settings Help"),
        "Zone A first, after the 1-cell R1 pad: {line}"
    );
    assert!(line.contains("│"), "separator present: {line}");
    assert!(line.contains("#1 ●"), "chip 1: {line}");
    assert!(line.contains("#2 ⠋"), "chip 2 (Running, 0ms): {line}");
    assert!(line.contains("#3 ●"), "chip 3: {line}");
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.cell_rects.len(), 3);
    // @step And the Running chip's glyph cell is styled magenta
    let chip2_rect = &layout.cell_rects[1];
    let glyph_x = chip2_rect.x + chip2_rect.width - 1;
    assert_eq!(
        buf[(glyph_x, 0)].fg,
        Color::Magenta,
        "Running glyph cell is magenta"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Each session status maps to its own glyph and color
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn each_session_status_maps_to_its_own_glyph_and_color() {
    // @step Given a MenuSnapshot with sessions in statuses Compacting, Paused and Interrupted
    let mut snap = snap(vec![
        status_chip(1, SessionStatus::Compacting, None, 0),
        status_chip(2, SessionStatus::Paused, None, 0),
        status_chip(3, SessionStatus::Interrupted, None, 0),
    ]);
    snap.clock_ms = 0;
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 120);
    // @step Then the chips render "↻", "!" and "✕"
    for (i, glyph) in ["↻", "!", "✕"].iter().enumerate() {
        let rect = &layout.cell_rects[i];
        assert_eq!(
            buf[(rect.x + rect.width - 1, 0)].symbol(),
            *glyph,
            "chip {i} glyph"
        );
    }
    // @step And the glyphs are styled cyan, yellow and red respectively
    let colors = [Color::Cyan, Color::Yellow, Color::Red];
    for (i, color) in colors.iter().enumerate() {
        let rect = &layout.cell_rects[i];
        assert_eq!(
            buf[(rect.x + rect.width - 1, 0)].fg,
            *color,
            "chip {i} glyph color"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A single open session shows the #1 prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_single_open_session_shows_the_1_prefix() {
    // @step Given a MenuSnapshot with exactly 1 open session (Idle)
    let snap = snap(vec![status_chip(1, SessionStatus::Idle, None, 0)]);
    // @step When the bar is rendered
    let (buf, _) = render(&snap, 80);
    // @step Then Zone B reads "#1 ●" (the prefix paints because n >= 1, SessionHeader parity)
    assert!(row(&buf, 80).contains("#1 ●"));
    assert_eq!(index_prefix((1, 1)), "#1 ");
    assert_eq!(index_prefix((0, 0)), "", "n == 0 paints no prefix");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The Running chip's braille glyph advances with the clock
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_running_chips_braille_glyph_advances_with_the_clock() {
    // @step Given a MenuSnapshot with 1 open session (Running)
    // @step When the bar is rendered at clock_ms 0 and again at clock_ms 80
    let at_0 = status_chip(1, SessionStatus::Running, None, 0);
    let at_80 = status_chip(1, SessionStatus::Running, None, 80);
    // @step Then the chip's glyph is the first braille frame at 0ms and the second frame at 80ms
    assert_eq!(at_0.glyph, "⠋", "frame 0");
    assert_eq!(at_80.glyph, "⠙", "frame 1 (80 ms cadence)");
    let at_240 = status_chip(1, SessionStatus::Running, None, 240);
    assert_eq!(at_240.glyph, "⠸", "frame 3");
    assert_eq!(at_0.glyph_style.fg, Some(Color::Magenta));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cleared sessions produce no chip
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cleared_sessions_produce_no_chip() {
    // @step Given a MenuSnapshot whose open session list contains a session in the Cleared status
    let inputs = [
        ChipInput {
            index: (1, 2),
            status: SessionStatus::Cleared,
            wu_id: None,
            active: false,
        },
        ChipInput {
            index: (2, 2),
            status: SessionStatus::Idle,
            wu_id: None,
            active: false,
        },
    ];
    // @step When the chip list is built
    let chips = build_chips(&inputs, 0);
    // @step Then no chip is produced for the Cleared session
    assert_eq!(chips.len(), 1, "the Cleared session is dropped");
    assert_eq!(chips[0].index, (2, 2), "only the open session remains");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A chip shows its work-unit id when width allows
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_chip_shows_its_work_unit_id_when_width_allows() {
    // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    let snap = snap(vec![status_chip(
        1,
        SessionStatus::Idle,
        Some("MENU-001"),
        0,
    )]);
    // @step And a 200-column render area
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 200);
    // @step Then the chip reads "#1 MENU-001 ●"
    assert_eq!(layout.level, 0, "wide area keeps the WU suffix");
    assert!(
        row(&buf, 200).contains("#1 MENU-001 ●"),
        "chip with WU id: {}",
        row(&buf, 200)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The focused menu item paints inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_focused_menu_item_paints_inverse_video() {
    // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
    let mut snap = snap(vec![chip(0, None)]);
    snap.focus = Some(MenuFocus::Item(0));
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80);
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The focused chip paints inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_focused_chip_paints_inverse_video() {
    // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
    let mut snap = snap(vec![chip(0, None), chip(1, None)]);
    snap.focus = Some(MenuFocus::ZoneB(1));
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80);
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: No focus paints no highlight
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn no_focus_paints_no_highlight() {
    // @step Given a MenuSnapshot with MenuFocus None
    let snap = snap(vec![chip(0, None), chip(1, None)]);
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The row paints 1-cell padding and a dark background on every cell
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_row_paints_1_cell_padding_and_a_dark_background_on_every_cell() {
    // @step Given a 40-column render area
    // @step When the bar is rendered
    let snap = snap(vec![chip(0, None)]);
    let (buf, _) = render(&snap, 40);
    // @step Then cell 0 and the last cell are background-only
    assert_eq!(buf[(0, 0)].symbol(), " ", "cell 0 is the pad");
    assert_eq!(buf[(39, 0)].symbol(), " ", "last cell is the pad");
    // @step And every cell of the row has the #333333 background
    for x in 0..40u16 {
        assert_eq!(buf[(x, 0)].bg, MENU_BAR_BG, "cell {x} missing bg");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An empty chip list omits the separator and Zone B
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn an_empty_chip_list_omits_the_separator_and_zone_b() {
    // @step Given a MenuSnapshot with no open sessions
    let snap = MenuSnapshot::default();
    // @step When the bar is rendered
    let (buf, layout) = render(&snap, 80);
    // @step Then the row contains the menu items but no "|" separator and no chips
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    let line = row(&buf, 80);
    assert!(!line.contains("│"));
    assert!(line.starts_with(" Kanban Tools Settings Help"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Tight width drops WU ids before folding chips
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn tight_width_drops_wu_ids_before_folding_chips() {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let snap = snap(vec![
        status_chip(1, SessionStatus::Idle, Some("MENU-001-LONGID"), 0),
        status_chip(2, SessionStatus::Idle, Some("MENU-002-LONGID"), 0),
        status_chip(3, SessionStatus::Idle, Some("MENU-003-LONGID"), 0),
        status_chip(4, SessionStatus::Idle, Some("MENU-004-LONGID"), 0),
    ]);
    // @step When the bar is rendered into a 52-column area
    // (MENU-008: Zone A is now 26 cells wide — "Kanban Tools Settings
    // Help" — so the old 38-col fixture would truncate to Zone A only;
    // 52 keeps level 2: WU ids dropped + the 4th chip folded.)
    let (buf, layout) = render(&snap, 52);
    // @step Then no chip shows a work-unit id suffix
    assert!(
        layout.level >= 1,
        "WU ids dropped first (got level {})",
        layout.level
    );
    let line = row(&buf, 52);
    assert!(!line.contains("MENU-001-LONGID"));
    assert!(!line.contains("MENU-004-LONGID"));
    // @step And the 4th chip is folded into a "+1" marker after the first 3
    assert!(
        layout.level >= 2,
        "level {} folds beyond the 3rd chip",
        layout.level
    );
    let folds = layout
        .cells
        .iter()
        .filter(|c| matches!(c, DisplayCell::Fold { .. }))
        .count();
    assert_eq!(folds, 1, "a single +N marker");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Extremely tight width degrades to the absolute minimum
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn extremely_tight_width_degrades_to_the_absolute_minimum() {
    // @step Given a MenuSnapshot with 4 open sessions
    let snap = snap(vec![
        status_chip(1, SessionStatus::Idle, Some("MENU-001"), 0),
        status_chip(2, SessionStatus::Idle, Some("MENU-002"), 0),
        status_chip(3, SessionStatus::Idle, Some("MENU-003"), 0),
        status_chip(4, SessionStatus::Idle, Some("MENU-004"), 0),
    ]);
    // @step When the bar is rendered into a 30-column area
    // @step Then the row still fits within the area width
    let Some(layout) = menu_bar_layout(Rect::new(0, 0, 30, 1), &snap) else {
        // Even the absolute minimum does not fit at 30 cols with these
        // WU ids → painting nothing is the safe degradation.
        return;
    };
    let last = layout
        .cell_rects
        .last()
        .or_else(|| layout.item_rects.last());
    assert!(
        last.is_some_and(|r| r.x + r.width <= 30),
        "rightmost painted cell stays inside the area"
    );
    // @step And it shows at least the "Kanban" and "Help" items
    let buf = paint_only(&snap, 30);
    let line = row(&buf, 30);
    if layout.level < 4 {
        assert!(
            line.starts_with(" Kanban Tools Settings Help"),
            "Zone A always survives: {line}"
        );
    }
}

proptest::proptest! {
    #[test]
    fn painted_row_never_exceeds_the_area_width(
        width in 3u16..=400,
        chip_count in 0usize..=24,
    ) {
        let chips: Vec<MenuChip> = (0..chip_count)
            .map(|i| status_chip(i + 1, SessionStatus::Idle, Some("A-BIG-UNIT-ID"), 0))
            .collect();
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
fn a_zero_width_or_zero_height_area_paints_nothing() {
    // @step Given a render area with width 0 or height 0
    let snap = snap(vec![chip(0, None)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    // @step When the bar is rendered
    // @step Then the buffer is unchanged
    assert!(paint_menu_bar(Rect::new(0, 0, 0, 1), &mut buf, &snap, &Theme::default()).is_none());
    assert!(paint_menu_bar(Rect::new(0, 0, 10, 0), &mut buf, &snap, &Theme::default()).is_none());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown panel is anchored under its menu item
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_panel_is_anchored_under_its_menu_item() {
    // @step Given a MenuSnapshot with the Kanban dropdown open at cursor 1
    // (Kanban = category 0; the item x is the 1-cell R1 pad in a
    // standard 80x24 bar)
    let item_x: u16 = 1;
    // @step When the dropdown is rendered
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), item_x, 0, 0).expect("panel");
    // @step Then the panel's x equals the Kanban item's x and its y is one row below the bar
    assert_eq!(panel.x, item_x);
    assert_eq!(panel.y, 1, "one row below the bar");
    // @step And its height is 6 rows (4 entries + 2 border rows)
    assert_eq!(panel.height, 6, "4 entries + 2 border rows");
    // @step And it has no title row and no footer row
    // (the height is exactly entries + 2; the width is
    // max(24, widest_row + 4) with the widest Kanban row at 48)
    assert_eq!(panel.width, 52, "max(24, 48 + 4) — no title/footer padding");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dropdown rows show a key-hint column, label and dim description
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dropdown_rows_show_a_key_hint_column_label_and_dim_description() {
    // @step Given the Actions dropdown rendered open
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 1, 0, 0).expect("panel");
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown cursor row is inverse-video
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_cursor_row_is_inverse_video() {
    // @step Given the Actions dropdown open with the cursor on row 1
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 1, 0, 0).expect("panel");
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown left-clamps at the terminal right edge
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_left_clamps_at_the_terminal_right_edge() {
    // @step Given a Help item positioned near the terminal's right edge
    // @step When its dropdown is rendered
    let panel = dropdown_rect(Rect::new(0, 0, 80, 24), 40, 0, 0).expect("panel");
    // @step Then the panel is shifted left so it never paints past the terminal width
    assert!(
        panel.x + panel.width <= 80,
        "panel stays inside: x {} w {}",
        panel.x,
        panel.width
    );
    assert!(panel.x < 40, "shifted left from the item");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The dropdown bottom-clips with an ellipsis when the terminal is too short
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_dropdown_bottom_clips_with_an_ellipsis_when_the_terminal_is_too_short() {
    // @step Given a short render area (5 rows below the bar)
    let panel = dropdown_rect(Rect::new(0, 0, 80, 6), 1, 0, 0).expect("panel");
    assert_eq!(panel.height, 5);
    // @step When the Kanban dropdown (4 entries) is rendered
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    // @step Then only 2 entry rows plus a dim "⋯" indicator row are visible (3 content rows inside the borders)
    assert_eq!(visible_entry_rows(panel, 4), 2, "2 entry rows");
    let ellipsis_y = panel.y + 1 + 2;
    assert_eq!(buf[(panel.x + 1, ellipsis_y)].symbol(), "⋯");
    // @step And the panel never paints below the area
    assert!(panel.bottom() <= 6);
}

#[test]
fn the_dropdown_cursor_never_clips_out_of_a_bottom_clipped_panel() {
    // 5 rows below the bar: content = 3 rows → 2 entry rows + ⋯.
    let panel = dropdown_rect(Rect::new(0, 0, 80, 6), 1, 0, 0).expect("panel");
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
}

#[test]
fn no_space_below_the_bar_yields_no_panel() {
    assert!(dropdown_rect(Rect::new(0, 0, 80, 1), 1, 0, 0).is_none());
    assert!(dropdown_rect(Rect::new(0, 0, 10, 24), 1, 0, 0).is_none());
}

// ─────────────────────────────────────────────────────────────────────────
// Supporting ring-math + registry-pin tests (R3, R5, R6 geometry helpers)
// ─────────────────────────────────────────────────────────────────────────

/// 2 items + 3 chips, all in Zone B (non-mux shape).
fn ring_snap3() -> MenuSnapshot {
    snap(vec![chip(0, None), chip(1, None), chip(2, None)])
}

#[test]
fn ring_walks_items_then_zone_b_then_wraps_forward() {
    let s = ring_snap3();
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
}

#[test]
fn ring_walks_backwards_with_wrap() {
    let s = ring_snap3();
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), -1), MenuFocus::ZoneB(2));
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), -1), MenuFocus::Item(3));
}

#[test]
fn ring_follows_the_display_order_in_mux_mode() {
    // Mux: Zone B = Board view, chip 1, chip 0, chip 2 (pane order).
    let chips = vec![chip(0, None), chip(1, None), chip(2, None)];
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
}

#[test]
fn empty_zone_b_ring_wraps_items_onto_themselves() {
    let s = MenuSnapshot::default();
    assert_eq!(s.advance(None, 1), MenuFocus::Item(0));
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), 1), MenuFocus::Item(1));
    assert_eq!(s.advance(Some(MenuFocus::Item(3)), 1), MenuFocus::Item(0));
}

#[test]
fn dropdown_cursor_clamps_into_the_category_range() {
    let s = MenuSnapshot::default();
    assert_eq!(s.dropdown_cursor(0, 3), 3, "Kanban has 4 rows");
    assert_eq!(s.dropdown_cursor(0, 4), 0, "wraps past the end");
    assert_eq!(s.dropdown_cursor(3, 1), 1, "Help has 2 rows");
    assert_eq!(s.dropdown_cursor(3, 2), 0, "Help wraps");
}

#[test]
fn chip_for_cell_resolves_only_chip_cells() {
    let s = ring_snap3();
    assert_eq!(s.chip_for_cell(0), Some(0));
    assert_eq!(s.chip_for_cell(2), Some(2));
    assert_eq!(s.chip_for_cell(9), None, "out of range");
}

#[test]
fn open_dropdown_marks_its_item_active() {
    let mut snap = snap(vec![chip(0, None)]);
    snap.open_menu = Some((3, 0));
    let (buf, layout) = render(&snap, 80);
    let help_x = layout.item_rects[3].x;
    assert_eq!(buf[(help_x, 0)].fg, Color::Black);
}

#[test]
fn wu_ids_count_toward_width_only_at_level_zero() {
    let snap = snap(vec![status_chip(
        1,
        SessionStatus::Idle,
        Some("MENU-001"),
        0,
    )]);
    let full = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).unwrap();
    assert_eq!(full.level, 0);
    assert_eq!(
        full.cell_rects[0].width,
        3 + 8 + 1 + 1,
        "#1 prefix + WU id + space + glyph"
    );
    // 40 cols: the level-0 row (26 Zone A + separator + 13 chip = 42
    // inner cells) no longer fits, so the WU id suffix is the first
    // thing dropped (R6 step 1).
    let tight = menu_bar_layout(Rect::new(0, 0, 40, 1), &snap).unwrap();
    assert_eq!(tight.level, 1, "WU id dropped first");
}

#[test]
fn full_width_keeps_level_zero_with_every_cell() {
    let snap = snap(vec![chip(0, None), chip(1, None), chip(2, None)]);
    let layout = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).expect("layout for 120-col area");
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.item_rects.len(), 4);
    assert_eq!(layout.cell_rects.len(), 3);
    assert!(layout.separator.is_some());
}

#[test]
fn very_tight_width_folds_chips_beyond_three() {
    let chips = (0..6).map(|i| chip(i, None)).collect();
    let layout = menu_bar_layout(Rect::new(0, 0, 30, 1), &snap(chips)).expect("layout");
    assert!(layout.level >= 2, "folding level used: {}", layout.level);
    let fold_count = layout
        .cells
        .iter()
        .filter(|c| matches!(c, DisplayCell::Fold { .. }))
        .count();
    assert_eq!(fold_count, 1, "a single +N marker");
}

#[test]
fn no_chips_is_the_full_level_with_no_zone_b() {
    // R6: with no chips the separator and Zone B are omitted
    // entirely — no truncation is needed, so the level stays 0.
    let layout = menu_bar_layout(Rect::new(0, 0, 80, 1), &MenuSnapshot::default()).unwrap();
    assert_eq!(layout.level, 0, "no chips → nothing to truncate");
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    assert!(layout.cell_rects.is_empty());
    assert_eq!(layout.item_rects.len(), 4, "Zone A still lays out");
}

#[test]
fn zero_or_tiny_area_yields_no_layout() {
    let s = snap(vec![chip(0, None)]);
    assert!(menu_bar_layout(Rect::new(0, 0, 0, 1), &s).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 10, 0), &s).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 2, 1), &s).is_none());
}

#[test]
fn key_hints_fit_the_four_cell_column() {
    assert_eq!(key("."), ".   ");
    assert_eq!(key("Esc"), "Esc ");
}

#[test]
fn truncate_keeps_short_text_and_caps_long_text() {
    assert_eq!(truncate("short", 10), "short");
    assert_eq!(truncate("abc", 0), "");
    assert_eq!(truncate("exactly-10", 10), "exactly-10");
}

#[test]
fn row_content_width_is_key_col_plus_label_plus_separator_plus_capped_description() {
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
}
