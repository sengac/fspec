#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn chip(i: usize) -> MenuChip {
    MenuChip {
        index: (i + 1, 3),
        glyph: "●".to_string(),
        glyph_style: Default::default(),
        wu_id: None,
        active: false,
        bot: false,
    }
}

/// 2 items + 3 chips, all in Zone B (non-mux shape).
fn snap3() -> MenuSnapshot {
    let chips = vec![chip(0), chip(1), chip(2)];
    MenuSnapshot {
        zone_b: vec![ZoneBCell::Chip(0), ZoneBCell::Chip(1), ZoneBCell::Chip(2)],
        chips,
        ..Default::default()
    }
}

mod part_1;
