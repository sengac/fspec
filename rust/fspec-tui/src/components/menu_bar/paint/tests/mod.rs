#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

use crate::components::menu_bar::chips::MenuChip;

use ratatui::backend::TestBackend;

fn chip(i: usize, wu: Option<&'static str>) -> MenuChip {
    MenuChip {
        index: (i + 1, 3),
        glyph: "●".to_string(),
        glyph_style: Default::default(),
        wu_id: wu.map(str::to_string),
        active: false,
        bot: false,
    }
}

fn render(snap: &MenuSnapshot, w: u16) -> (Buffer, MenuLayout) {
    let area = Rect::new(0, 0, w, 1);
    let _ = TestBackend::new(w, 4);
    let mut buf = Buffer::empty(area);
    let layout =
        paint_menu_bar(area, &mut buf, snap, &Theme::default()).expect("layout for non-zero area");
    (buf, layout)
}

fn row(buf: &Buffer, w: u16) -> String {
    (0..w)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn snap(chips: Vec<MenuChip>) -> MenuSnapshot {
    let zone_b = chips
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    MenuSnapshot {
        zone_b,
        chips,
        ..Default::default()
    }
}

mod part_1;
