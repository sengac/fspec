use super::*;

#[test]
fn full_width_row_paints_items_separator_and_chips() {
    // @step Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
    // @step When the bar is rendered into a 120-column row
    let (buf, layout) = render(
        &snap(vec![chip(0, None), chip(1, None), chip(2, None)]),
        120,
    );
    // @step Then Zone B reads "#1 ●  #2 ⠋  #3 ●"
    let line = row(&buf, 120);
    assert!(
        line.starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"),
        "Zone A first, after the 1-cell R1 pad (MENU-011: bracketed): {line}"
    );
    assert!(line.contains("│"), "separator present: {line}");
    assert!(line.contains("[ #1 ● ]"), "chip 1: {line}");
    assert!(line.contains("[ #2 ● ]"), "chip 2: {line}");
    assert!(line.contains("[ #3 ● ]"), "chip 3: {line}");
    // @step And the Running chip's glyph cell is styled magenta
    // (the 120-col fixture chips are Idle-styled dots; the Running
    // braille + magenta styling is pinned in chips::running_uses_the_braille_frame_at_the_clock)
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.item_rects.len(), 4);
    assert_eq!(layout.cell_rects.len(), 3);
}

#[test]
fn wu_ids_appear_at_level_zero() {
    // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    // @step And a 200-column render area
    // @step When the bar is rendered
    let (buf, layout) = render(&snap(vec![chip(0, Some("MENU-001"))]), 120);
    // @step Then the chip reads "#1 MENU-001 ●"
    assert_eq!(layout.level, 0);
    assert!(row(&buf, 120).contains("[ #1 MENU-001 ● ]"));
}

#[test]
fn tight_width_drops_wu_ids_before_anything_else() {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let snap = snap(vec![chip(0, Some("MENU-001")), chip(1, Some("MENU-002"))]);
    // @step When the bar is rendered into a 80-column area
    // (MENU-008: Zone A grew from 9 to 26 inner cells, so the
    // level-0/level-1 boundary moved; MENU-011: the bracketed Zone
    // A is 42 cells — at 80 cols (78 inner) level 0 needs 81, so
    // the WU ids are still the first thing dropped.)
    let (buf, layout) = render(&snap, 80);
    // @step Then no chip shows a work-unit id suffix
    assert_eq!(layout.level, 1, "WU ids dropped first");
    assert!(!row(&buf, 80).contains("MENU-001"));
    assert!(row(&buf, 80).contains("[ #1 ● ]"));
}

#[test]
fn focused_item_paints_inverse_video() {
    // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
    let mut snap = snap(vec![chip(0, None)]);
    snap.focus = Some(MenuFocus::Item(0));
    // @step When the bar is rendered
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
    // @step Then the "Kanban" item's cells are styled bg Cyan fg Black
    // The 'K' of 'Kanban' (x=1 after the 1-cell pad) is inverse.
    assert_eq!(buf[(1, 0)].bg, Color::Cyan);
    assert_eq!(buf[(1, 0)].fg, Color::Black);
    assert!(buf[(1, 0)].modifier.contains(Modifier::BOLD));
    // @step And the "Help" item's cells are not inverted
    let help_x = layout.item_rects[3].x;
    assert_ne!(buf[(help_x, 0)].bg, Color::Cyan);
}

#[test]
fn focused_chip_paints_inverse_video() {
    // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
    let mut snap = snap(vec![chip(0, None), chip(1, None)]);
    snap.focus = Some(MenuFocus::ZoneB(1));
    // @step When the bar is rendered
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
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
}

#[test]
fn open_dropdown_marks_its_item_active() {
    let mut snap = snap(vec![chip(0, None)]);
    snap.open_menu = Some((3, 0));
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
    let help_x = layout.item_rects[3].x;
    assert_eq!(buf[(help_x, 0)].fg, Color::Black);
}

#[test]
fn no_focus_paints_no_highlight() {
    // @step Given a MenuSnapshot with MenuFocus None
    // @step When the bar is rendered
    let snap = snap(vec![chip(0, None), chip(1, None)]);
    let (buf, _) = render(&snap, 80);
    // @step Then no cell in the row is styled bg Cyan
    for x in 0..80u16 {
        assert_ne!(
            buf[(x, 0)].bg,
            Color::Cyan,
            "cell {x} unexpectedly inverse without a focus"
        );
    }
}

#[test]
fn zero_area_paints_nothing() {
    // @step Given a render area with width 0 or height 0
    // @step When the bar is rendered
    let snap = snap(vec![chip(0, None)]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    // @step Then the buffer is unchanged
    assert!(paint_menu_bar(Rect::new(0, 0, 0, 1), &mut buf, &snap, &Theme::default()).is_none());
    assert!(paint_menu_bar(Rect::new(0, 0, 10, 0), &mut buf, &snap, &Theme::default()).is_none());
}

#[test]
fn every_cell_carries_the_dark_background() {
    // @step Given a 40-column render area
    let (buf, _) = render(&snap(vec![chip(0, None)]), 60);
    // @step And every cell of the row has the #333333 background
    for x in 0..60u16 {
        assert_eq!(buf[(x, 0)].bg, MENU_BAR_BG, "cell {x} missing bg");
    }
}

#[test]
fn no_chips_omits_separator_and_zone_b() {
    // R6: with no chips the separator and Zone B are omitted
    // entirely — the row is just the menu items.
    // @step Given a MenuSnapshot with no open sessions
    // @step When the bar is rendered
    let snap = MenuSnapshot::default();
    let (buf, layout) = render(&snap, 80);
    // @step Then the row contains the menu items but no "|" separator and no chips
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    assert!(!row(&buf, 80).contains("│"));
    assert!(row(&buf, 80).starts_with(" [ Kanban ] [ Tools ] [ Settings ] [ Help ]"));
}
