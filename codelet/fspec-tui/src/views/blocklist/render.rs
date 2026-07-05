//! RPC-056 / BLOCK-008 / BLOCK-009 — BlocklistView rendering.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-view-scrolling.feature
//! Feature: spec/features/blocklist-view-framing.feature
//!
//! Uses the shared `full_screen_shell` scaffold (count-title
//! `Blocklist Rules (N rules)` + reference-parity footer). The body is a
//! `[Percentage(50), Length(1), Percentage(50)]` split with a shared
//! `diff_common::render_vertical_divider` between the windowed left list
//! pane (scrollbar gutter + `Showing X-Y of N` from BLOCK-008) and the
//! right details pane.

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use super::panes::{render_empty, render_left_pane, render_right_pane};
use super::BlocklistView;
use crate::views::diff_common::render_vertical_divider;
use crate::views::full_screen_shell::render_full_screen_scaffold;

const FOOTER_HINT: &str =
    "↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | Esc: Close";

impl BlocklistView {
    /// Paint the view into `area` using the shared full-screen scaffold.
    /// The header shows `Blocklist Rules (N rules)`; the body is a
    /// two-pane split (windowed list + details) separated by a shared
    /// vertical divider. The supplied `session_disabled` set drives the
    /// per-row glyph (●/○) and the right-pane Session Status field.
    ///
    /// Takes `&mut self` so the render pass can reconcile `visible_rows`
    /// and `scroll_offset` from the real body height (BLOCK-008),
    /// mirroring the `changed_files` idiom: the mutable rule list is
    /// moved out with `std::mem::take` before the FnOnce body closure and
    /// restored afterwards, so the closure only borrows the taken `Vec`
    /// plus scalars.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, session_disabled: &HashSet<String>) {
        let count = self.rules.len();
        let rules = std::mem::take(&mut self.rules);
        let selected_index = self.selected_index;
        let mut scroll_offset = self.scroll_offset;
        let mut visible_rows = self.visible_rows;
        render_full_screen_scaffold(
            area,
            buf,
            "Blocklist Rules",
            count,
            "rules",
            FOOTER_HINT,
            |body, buf| {
                if rules.is_empty() {
                    visible_rows = 0;
                    scroll_offset = 0;
                    render_empty(body, buf);
                    return;
                }
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50),
                        Constraint::Length(1),
                        Constraint::Percentage(50),
                    ])
                    .split(body);
                render_vertical_divider(panes[1], buf);
                render_left_pane(
                    panes[0],
                    buf,
                    &rules,
                    selected_index,
                    &mut scroll_offset,
                    &mut visible_rows,
                    session_disabled,
                );
                render_right_pane(panes[2], buf, rules.get(selected_index), session_disabled);
            },
            None,
        );
        self.rules = rules;
        self.scroll_offset = scroll_offset;
        self.visible_rows = visible_rows;
    }
}
