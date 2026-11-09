use super::*;

#[test]
fn ring_walks_items_then_zone_b_then_wraps_forward() {
    // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
    let s = snap3();
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
    // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
    let s = snap3();
    assert_eq!(s.advance(Some(MenuFocus::Item(0)), -1), MenuFocus::ZoneB(2));
    assert_eq!(s.advance(Some(MenuFocus::ZoneB(0)), -1), MenuFocus::Item(3));
}

#[test]
fn ring_follows_the_display_order_in_mux_mode() {
    // Mux: Zone B = Board view, chip 1, chip 0, chip 2 (pane order).
    let chips = vec![chip(0), chip(1), chip(2)];
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
    assert_eq!(s.dropdown_cursor(0, 4), 4, "Kanban has 5 rows");
    assert_eq!(s.dropdown_cursor(0, 5), 0, "wraps past the end");
    assert_eq!(s.dropdown_cursor(3, 1), 1, "Help has 2 rows");
    assert_eq!(s.dropdown_cursor(3, 2), 0, "Help wraps");
}

#[test]
fn chip_for_cell_resolves_only_chip_cells() {
    let s = snap3();
    assert_eq!(s.chip_for_cell(0), Some(0));
    assert_eq!(s.chip_for_cell(2), Some(2));
    assert_eq!(s.chip_for_cell(9), None, "out of range");
}
