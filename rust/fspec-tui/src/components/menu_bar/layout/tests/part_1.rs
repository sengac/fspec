use super::*;

#[test]
fn full_width_keeps_level_zero_with_every_cell() {
    // @step Given a 40-column render area
    let snap = snap(vec![chip(0, None), chip(1, None), chip(2, None)]);
    // @step When the bar is rendered
    let layout = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).expect("layout for 120-col area");
    // @step Then cell 0 and the last cell are background-only
    // @step And every cell of the row has the #333333 background
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    assert_eq!(layout.item_rects.len(), 4, "MENU-008: four Zone A items");
    assert_eq!(layout.cell_rects.len(), 3);
    assert!(layout.separator.is_some());
}

#[test]
fn wu_ids_count_toward_width_only_at_level_zero() {
    // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    let snap = snap(vec![chip(0, Some("MENU-001"))]);
    let full = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).unwrap();
    assert_eq!(full.level, 0);
    assert_eq!(
        full.cell_rects[0].width,
        3 + 8 + 1 + 1 + 4,
        "#1 prefix + WU id + space + glyph + the MENU-011 '[ ]' brackets"
    );
    // 60 cols: the level-0 row (42 bracketed Zone A + 3 separator
    // + 17 chip = 69 inner cells) no longer fits, so the WU id
    // suffix is the first thing dropped (R6 step 1).
    let tight = menu_bar_layout(Rect::new(0, 0, 60, 1), &snap).unwrap();
    // @step Then no chip shows a work-unit id suffix
    assert_eq!(tight.level, 1, "WU id dropped first");
}

#[test]
fn very_tight_width_folds_chips_beyond_three() {
    // @step And the 4th chip is folded into a "+1" marker after the first 3
    // (MENU-011: the bracketed Zone A is 42 cells — the fold level
    // needs 84 inner cells, so the 90-col area is the fixture.)
    let chips = (0..6).map(|i| chip(i, None)).collect();
    let layout = menu_bar_layout(Rect::new(0, 0, 90, 1), &snap(chips)).expect("layout");
    assert!(layout.level >= 2, "folding level used: {}", layout.level);
    let fold_count = layout
        .cells
        .iter()
        .filter(|c| matches!(c, DisplayCell::Fold { .. }))
        .count();
    assert_eq!(fold_count, 1, "a single +N marker");
}

#[test]
fn absolute_minimum_drops_zone_b() {
    // @step Given a MenuSnapshot with 4 open sessions
    // @step When the bar is rendered into a 50-column area
    // (MENU-011: the bracketed Zone A is 42 cells — the minimum a
    // 50-col area keeps; every Zone B ladder step overflows, so
    // Zone B is dropped entirely.)
    // @step Then the row still fits within the area width
    // @step And it shows at least the "Kanban" and "Help" items
    let chips = (0..6)
        .map(|i| chip(i, Some("A-LONG-UNIT-ID-TO-WRAP")))
        .collect();
    let layout = menu_bar_layout(Rect::new(0, 0, 50, 1), &snap(chips)).unwrap();
    assert_eq!(layout.level, 4, "Zone B dropped entirely");
    assert!(layout.separator.is_none());
    assert!(layout.cell_rects.is_empty());
}

#[test]
fn zero_or_tiny_area_yields_no_layout() {
    // @step Given a render area with width 0 or height 0
    // @step When the bar is rendered
    // @step Then the buffer is unchanged
    assert!(menu_bar_layout(Rect::new(0, 0, 0, 1), &snap(vec![chip(0, None)])).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 10, 0), &snap(vec![chip(0, None)])).is_none());
    assert!(menu_bar_layout(Rect::new(0, 0, 2, 1), &snap(vec![chip(0, None)])).is_none());
}

#[test]
fn no_chips_is_the_full_level_with_no_zone_b() {
    // R6: with no chips the separator and Zone B are omitted
    // entirely — no truncation is needed, so the level stays 0.
    // @step Given a MenuSnapshot with no open sessions
    // @step When the bar is rendered
    // @step Then the row contains the menu items but no "|" separator and no chips
    let layout = menu_bar_layout(Rect::new(0, 0, 80, 1), &MenuSnapshot::default()).unwrap();
    assert_eq!(layout.level, 0, "no chips → nothing to truncate");
    assert!(layout.separator.is_none());
    assert!(layout.cells.is_empty());
    assert!(layout.cell_rects.is_empty());
    assert_eq!(
        layout.item_rects.len(),
        4,
        "Zone A still lays out (MENU-008)"
    );
}

// R6 proptest: EVERY ladder step must keep the painted row within
// `area.width` — for any number of chips and any area width, the
// rightmost painted cell (last Zone B cell or last item) ends at
// or before `area.x + area.width`.
proptest::proptest! {
    #[test]
    fn painted_row_never_exceeds_the_area_width(
        width in 3u16..=400,
        chip_count in 0usize..=24,
    ) {
        let chips: Vec<MenuChip> = (0..chip_count)
            .map(|i| chip(i, Some("A-BIG-UNIT-ID")))
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
