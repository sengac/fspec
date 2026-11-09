//! MUX-013 — the mux bar's New Bot / New Agent / Close buttons and the
//! bar rows they wrap onto.
//!
//! Feature: spec/features/mux-bar-new-bot-new-agent-and-close-buttons.feature
//!
//! The buttons are the agent view's Zone C (`[ New Bot ]`, `[ New Agent ]`,
//! `[ Close ]`); `[ Close ]` shows only while the focused pane shows a bot
//! or an agent (the board view has no Close either). When Zone A, the gap
//! and all three buttons do not fit on the bar row, the buttons wrap onto
//! the rows below it, right-aligned (TUI-131: nothing is cut off). The row
//! count depends only on the width, so the panes never move when the focus
//! or the open sessions change.
//!
//! Painting reuses the shared MENU-001 painter twice per row: a
//! buttons-only snapshot over the full row, then Zone A/B in the room left
//! of the buttons (row 0) — the painter itself is unchanged.

use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use crate::components::menu_bar::items::{MenuAction, CATEGORIES};
use crate::components::menu_bar::layout::ZONE_C_GAP;
use crate::components::menu_bar::ZoneCButton;
use crate::components::menu_bar::{paint_menu_bar, MenuFocus, MenuLayout, MenuSnapshot};
use crate::theme::Theme;

use super::MultiplexLayout;

/// The mux bar's buttons, the agent view's labels and order.
pub(crate) const MUX_ZONE_C: &[ZoneCButton] = &[
    ZoneCButton {
        label: "[ New Bot ]",
        action: MenuAction::NewBot,
    },
    ZoneCButton {
        label: "[ New Agent ]",
        action: MenuAction::NewAgent,
    },
    ZoneCButton {
        label: "[ Close ]",
        action: MenuAction::CloseAgent,
    },
];

/// The buttons without `[ Close ]` (no session in the focused pane).
const START_BUTTONS: usize = 2;

fn buttons_width(buttons: &[ZoneCButton]) -> usize {
    buttons.iter().map(|b| b.label.width()).sum::<usize>()
        + buttons.len().saturating_sub(1) * ZONE_C_GAP
}

fn zone_a_width() -> usize {
    CATEGORIES
        .iter()
        .map(|c| c.bracketed_label().width())
        .sum::<usize>()
        + CATEGORIES.len().saturating_sub(1)
}

/// The rows below row 0 the buttons wrap onto (ranges into
/// [`MUX_ZONE_C`]), greedily filled; empty when every button fits on
/// row 0 beside Zone A. Decided with all three buttons, so it depends on
/// the width alone.
pub(crate) fn wrapped_rows(width: u16) -> Vec<Range<usize>> {
    let inner = usize::from(width).saturating_sub(2);
    if zone_a_width() + ZONE_C_GAP + buttons_width(MUX_ZONE_C) <= inner {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut start = 0;
    while start < MUX_ZONE_C.len() {
        let mut end = start + 1;
        while end < MUX_ZONE_C.len()
            && ZONE_C_GAP + buttons_width(&MUX_ZONE_C[start..=end]) <= inner
        {
            end += 1;
        }
        rows.push(start..end);
        start = end;
    }
    rows
}

/// The number of rows the bar takes at `width` (1 + the wrapped rows).
pub(crate) fn bar_height(width: u16) -> u16 {
    1 + wrapped_rows(width).len() as u16
}

impl MultiplexLayout {
    /// MUX-013: the buttons the bar shows now — `[ Close ]` only while the
    /// focused pane shows a bot or an agent.
    pub fn zone_c(&self) -> &'static [ZoneCButton] {
        if self.focused_session_id().is_some() {
            MUX_ZONE_C
        } else {
            &MUX_ZONE_C[..START_BUTTONS]
        }
    }

    /// MUX-013: the bar rows of the last painted frame.
    pub fn menu_bar_rect(&self) -> Option<Rect> {
        self.menu_bar_rect
    }

    /// MUX-013: the painted button rects (`zone_c()` order) of the last
    /// frame.
    pub fn menu_zone_c_rects(&self) -> Option<&[Rect]> {
        self.menu_zone_c_rects.as_deref()
    }

    pub(super) fn cache_bar_rows(&mut self, bar: Rect, zone_c: Vec<Rect>) {
        self.menu_bar_rect = Some(bar);
        self.menu_zone_c_rects = Some(zone_c);
    }
}

/// A snapshot that paints only `buttons` (offset `first` into the full
/// slice, so the ring focus maps onto them).
fn buttons_snapshot(
    snap: &MenuSnapshot,
    buttons: &'static [ZoneCButton],
    first: usize,
) -> MenuSnapshot {
    let focus = match snap.focus {
        Some(MenuFocus::ZoneC(i)) if (first..first + buttons.len()).contains(&i) => {
            Some(MenuFocus::ZoneC(i - first))
        }
        _ => None,
    };
    MenuSnapshot {
        zone_a: &[],
        focus,
        open_menu: None,
        zone_b: Vec::new(),
        chips: Vec::new(),
        zone_c: buttons,
        clock_ms: snap.clock_ms,
    }
}

/// Paint the buttons `range` of `snap.zone_c` right-aligned on `row`;
/// returns their rects.
fn paint_buttons(
    row: Rect,
    buf: &mut Buffer,
    snap: &MenuSnapshot,
    range: Range<usize>,
    theme: &Theme,
) -> Vec<Rect> {
    let buttons = &snap.zone_c[range.clone()];
    paint_menu_bar(
        row,
        buf,
        &buttons_snapshot(snap, buttons, range.start),
        theme,
    )
    .map(|layout| layout.zone_c_rects)
    .unwrap_or_else(|| vec![Rect::new(row.x, row.y, 0, 1); range.len()])
}

/// Paint the bar for `snap` (its `zone_c` the shown buttons) into `bar`
/// (the rows [`bar_height`] gives) and return the Zone A/B layout plus
/// the button rects in `snap.zone_c` order. `None` when Zone A does not
/// fit.
pub(crate) fn paint_rows(
    bar: Rect,
    buf: &mut Buffer,
    snap: &MenuSnapshot,
    theme: &Theme,
) -> Option<(MenuLayout, Vec<Rect>)> {
    let row = |i: u16| Rect::new(bar.x, bar.y + i, bar.width, 1);
    let left_snap = MenuSnapshot {
        zone_c: &[],
        focus: snap.focus.filter(|f| !matches!(f, MenuFocus::ZoneC(_))),
        ..snap.clone()
    };
    let shown = snap.zone_c.len();
    let wrapped = wrapped_rows(bar.width);
    if wrapped.is_empty() {
        let rects = paint_buttons(row(0), buf, snap, 0..shown, theme);
        let room = (buttons_width(snap.zone_c) + ZONE_C_GAP) as u16;
        let left = Rect::new(bar.x, bar.y, bar.width.saturating_sub(room), 1);
        let layout = paint_menu_bar(left, buf, &left_snap, theme)?;
        return Some((layout, rects));
    }
    let layout = paint_menu_bar(row(0), buf, &left_snap, theme)?;
    let mut rects = Vec::with_capacity(shown);
    for (i, range) in wrapped.into_iter().enumerate() {
        let range = range.start.min(shown)..range.end.min(shown);
        let line = row(1 + i as u16);
        if range.is_empty() {
            // The row's buttons are not shown (no Close): keep the row.
            paint_menu_bar(line, buf, &buttons_snapshot(snap, &[], 0), theme);
            continue;
        }
        rects.extend(paint_buttons(line, buf, snap, range, theme));
    }
    Some((layout, rects))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn the_buttons_wrap_below_83_columns_only() {
        assert_eq!(bar_height(120), 1);
        assert_eq!(bar_height(80), 2);
        assert_eq!(wrapped_rows(80), vec![0..3]);
        assert_eq!(wrapped_rows(30), vec![0..2, 2..3]);
        assert_eq!(wrapped_rows(29), vec![0..1, 1..3]);
    }
}
