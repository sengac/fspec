use super::*;

#[test]
fn ring_walks_items_then_chips_then_wraps() {
    // @step Given a MultiplexLayout with 2 open sessions and the ring focus on Item(0)
    let mut m = layout(Some(MenuFocus::Item(0)), 2);
    // @step When the ring walks Right six stops
    walk(&mut m, 2, 1);
    walk(&mut m, 2, 1);
    walk(&mut m, 2, 1);
    walk(&mut m, 2, 1);
    walk(&mut m, 2, 1);
    walk(&mut m, 2, 1);
    // @step Then the focus wraps from the last chip back to Item(0)
    // (4 items + 2 chips = 6 ring stops)
    assert_eq!(m.menu_focus(), Some(MenuFocus::Item(0)));
}

#[test]
fn ring_walks_left_from_the_first_item_to_the_last_chip() {
    let mut m = layout(Some(MenuFocus::Item(0)), 2);
    walk(&mut m, 2, -1);
    assert_eq!(m.menu_focus(), Some(MenuFocus::ZoneB(1)));
}

#[test]
fn walk_into_an_item_reanchors_the_open_dropdown_at_row_0() {
    // @step Given the Kanban dropdown is open with the cursor on row 1
    let mut m = layout(Some(MenuFocus::Item(0)), 1);
    m.open_menu = Some((0, 1));
    // @step When the ring walks Right onto Tools
    walk(&mut m, 1, 1);
    // @step Then Tools has the focus and its dropdown is open at row 0
    assert_eq!(m.menu_focus(), Some(MenuFocus::Item(1)));
    assert_eq!(m.open_menu(), Some((1, 0)));
}

#[test]
fn walk_onto_a_zone_b_cell_closes_the_dropdown() {
    let mut m = layout(Some(MenuFocus::Item(0)), 1);
    m.open_menu = Some((0, 0));
    // @step When the ring walks Right four stops (past all four items) onto the chip
    walk(&mut m, 1, 1);
    walk(&mut m, 1, 1);
    walk(&mut m, 1, 1);
    walk(&mut m, 1, 1);
    assert_eq!(m.menu_focus(), Some(MenuFocus::ZoneB(0)));
    assert_eq!(m.open_menu(), None);
}

#[test]
fn open_or_close_toggles_the_same_item_and_reopens_others_at_row_0() {
    let mut m = layout(Some(MenuFocus::Item(1)), 0);
    m.menu_open_or_close(1);
    assert_eq!(m.open_menu(), Some((1, 0)));
    m.menu_open_or_close(1);
    assert_eq!(m.open_menu(), None, "the second tick closes");
    assert_eq!(m.menu_focus(), Some(MenuFocus::Item(1)));
    m.menu_open_or_close(0);
    assert_eq!(
        m.open_menu(),
        Some((0, 0)),
        "another item re-opens at row 0"
    );
}

#[test]
fn dropdown_cursor_wraps_within_the_category_entries() {
    let mut m = layout(Some(MenuFocus::Item(0)), 0);
    m.open_menu = Some((0, 4));
    m.menu_dropdown_cursor(1);
    assert_eq!(m.open_menu(), Some((0, 0)), "Kanban has 5 rows");
    m.menu_dropdown_cursor(-1);
    assert_eq!(m.open_menu(), Some((0, 4)));
}

#[test]
fn execute_resolves_the_registry_row_and_clears_the_bar() {
    let mut m = layout(Some(MenuFocus::Item(0)), 0);
    m.open_menu = Some((1, 1));
    let action = m.menu_execute_item(1, 1);
    assert_eq!(
        action,
        Some(crate::components::menu_bar::items::MenuAction::Checkpoints)
    );
    assert_eq!(m.menu_focus(), None, "the bar focus clears on execute");
    assert_eq!(m.open_menu(), None);
}

#[test]
fn reset_clears_every_bar_field() {
    let mut m = layout(Some(MenuFocus::Item(0)), 2);
    m.menu_bar_painted = true;
    m.menu_reset();
    assert_eq!(m.menu_focus(), None);
    assert_eq!(m.open_menu(), None);
    assert!(!m.menu_bar_painted());
    assert_eq!(m.menu_chips, 0);
}

#[test]
fn view_label_count_counts_the_non_agent_panes() {
    let mut m = MultiplexLayout::new();
    m.enable_default();
    m.set_layout(
        crate::views::multiplex::MuxLayout::columns(&[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Agent,
            MuxPaneKind::Checkpoints,
        ])
        .unwrap(),
    );
    assert_eq!(m.view_label_count(), 2, "Board + Ckpts labels");
}
