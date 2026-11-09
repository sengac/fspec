#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

use crate::components::menu_bar::chips::MenuChip;

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
