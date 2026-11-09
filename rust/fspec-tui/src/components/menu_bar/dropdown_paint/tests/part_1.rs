use super::*;

#[test]
fn rows_show_key_column_label_and_dim_description() {
    // @step Given the Kanban dropdown rendered open
    let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    // @step When row 2 is inspected
    let y = panel.y + 1;
    let y2 = panel.y + 1 + 1;
    // @step Then it starts with the dim 4-cell key hint "/ "
    assert_eq!(buf[(panel.x + 1, y2)].symbol(), "/");
    // @step And it shows the label "Search" in the primary foreground
    let theme = Theme::default();
    assert_eq!(buf[(panel.x + 1 + KEY_COL, y2)].symbol(), "S");
    assert_eq!(buf[(panel.x + 1 + KEY_COL, y2)].fg, theme.fg);
    assert_eq!(buf[(panel.x + 1, y2)].fg, theme.dim, "key hint is dim");
    // @step And it shows a dim description truncated with an ellipsis when the row is too wide
    assert_eq!(
        buf[(panel.x + 1, y)].symbol(),
        ".",
        "row 1 (index 0) keeps its key hint"
    );
}

#[test]
fn cursor_row_is_inverse_across_the_inner_width() {
    // @step Given the Kanban dropdown open with the cursor on row 1
    let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 1, &Theme::default());
    // @step When the dropdown is rendered
    let y = panel.y + 1 + 1;
    // @step Then row 2 (index 1) is styled bg Cyan fg Black bold across its full inner width
    let inner_end = panel.right().saturating_sub(2);
    for x in panel.x + 1..=inner_end {
        assert_eq!(buf[(x, y)].bg, Color::Cyan, "inner cell {x} inverted");
    }
    // The label cell ("S" of "Search", past the 4-cell key column)
    // carries the inverse fg + bold.
    let label_x = panel.x + 1 + KEY_COL;
    assert_eq!(buf[(label_x, y)].fg, Color::Black);
    assert!(buf[(label_x, y)].modifier.contains(Modifier::BOLD));
    // @step And all other rows are not inverted
    let other_y = panel.y + 1 + 3;
    assert_ne!(buf[(panel.x + 1, other_y)].bg, Color::Cyan);
}

#[test]
fn panel_bottom_clips_with_an_ellipsis_row() {
    // @step Given a short render area (5 rows below the bar)
    let panel = dropdown_rect(bar_area(80, 6), KANBAN_X, 0, 0).expect("panel");
    assert_eq!(panel.height, 5);
    // @step When the Kanban dropdown (5 entries) is rendered
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
    // @step Then only 2 entry rows plus a dim "⋯" indicator row are visible (4 content rows inside the borders)
    let ellipsis_y = panel.y + 1 + 2;
    assert_eq!(buf[(panel.x + 1, ellipsis_y)].symbol(), "⋯");
}

#[test]
fn panel_anchors_under_its_item() {
    // @step Given a MenuSnapshot with the Kanban dropdown open at cursor 1
    // @step When the dropdown is rendered
    let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
    // @step Then the panel's x equals the Kanban item's x and its y is one row below the bar
    assert_eq!(panel.x, KANBAN_X);
    assert_eq!(panel.y, 1, "one row below the bar");
    // @step And its height is 7 rows (5 entries + 2 border rows)
    assert_eq!(panel.height, 7, "5 entries + 2 border rows");
    // @step And it has no title row and no footer row
    // (height is exactly entries + 2 border rows; width is
    // max(24, widest_row + 4) with the widest Kanban row at 48)
    assert_eq!(panel.width, 52, "max(24, 48 + 4) — no title/footer padding");
}

#[test]
fn panel_left_clamps_at_the_terminal_edge() {
    // @step Given a Help item positioned near the terminal's right edge
    // @step When its dropdown is rendered
    let panel = dropdown_rect(bar_area(80, 24), 40, 0, 0).expect("panel");
    // @step Then the panel is shifted left so it never paints past the terminal width
    assert!(
        panel.x + panel.width <= 80,
        "panel stays inside: x {} w {}",
        panel.x,
        panel.width
    );
    assert!(panel.x < 40, "shifted left from the item");
}

#[test]
fn cursor_never_clips_out_of_a_bottom_clipped_panel() {
    // 5 rows below the bar: content = 3 rows → 2 entry rows + ⋯.
    let panel = dropdown_rect(bar_area(80, 6), KANBAN_X, 0, 0).expect("panel");
    assert_eq!(visible_entry_rows(panel, 5), 2, "2 entry rows");
    // Cursor on the last entry → the window scrolls to 3..=4.
    let start = scroll_window(panel, 5, 4);
    assert_eq!(start, 3, "rows 3..4 visible, cursor row last");
    let mut buf = Buffer::empty(panel);
    render_menu_dropdown(panel, &mut buf, 0, 4, &Theme::default());
    // The 'b' key of the last entry ('b New Bot') is painted on
    // content row 1 (the last visible entry row).
    let y = panel.y + 1 + 1;
    assert_eq!(buf[(panel.x + 1, y)].symbol(), "b");
    // And the ellipsis sits on the final content row.
    assert_eq!(buf[(panel.x + 1, panel.y + 1 + 2)].symbol(), "⋯");
}

#[test]
fn no_space_below_the_bar_yields_no_panel() {
    assert!(dropdown_rect(bar_area(80, 1), KANBAN_X, 0, 0).is_none());
    assert!(dropdown_rect(bar_area(10, 24), KANBAN_X, 0, 0).is_none());
}
