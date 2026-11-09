//! RPC-448: the board's daemon switcher — the "Daemon: <name>" label on
//! header row 1 (after the workspace label), the dropdown beneath it and
//! its keys. `s` opens it; while open Up/Down (k/j) move, Enter switches,
//! Esc closes. Chained from `workspace_dropdown` so `board.rs` stays put.
//!
//! Feature: spec/features/tui-daemon-switcher.feature
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature
//!
//! TUI-123: like the workspace dropdown, a click on the label opens the
//! switcher, a click on a row chooses it and a click elsewhere closes it.
//! An unreadable daemon list is shown inside the panel (never as an empty
//! list), and labels too long for the terminal are cut with `…`.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Clear, Widget};

use crate::components::daemon_form_dialog::wrap_text;
use crate::components::{Action, EventResult};
use crate::mouse::rect_contains;
use crate::store::BoardStore;
use crate::theme::Theme;

use super::BoardView;

/// TUI-123: the switcher's keys, on its bottom border.
const SWITCHER_KEYS: &str = " Enter connect · a add · e edit · d remove · Esc close ";

/// Cells between the workspace label and the daemon label.
const GAP: u16 = 3;

const LABEL_PREFIX: &str = "Daemon: ";
const LABEL_SUFFIX: &str = " ▾";

/// `text` cut to `width` characters, ending in `…` when cut.
fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
    if width > 0 {
        cut.push('…');
    }
    cut
}

/// The label as painted in `width` cells (the name is cut first).
fn label(store: &BoardStore, width: usize) -> Option<String> {
    let name = store.connected_daemon()?;
    let frame = LABEL_PREFIX.chars().count() + LABEL_SUFFIX.chars().count();
    let full = format!("{LABEL_PREFIX}{name}{LABEL_SUFFIX}");
    if full.chars().count() <= width || width < frame + 2 {
        return Some(fit(&full, width));
    }
    Some(format!(
        "{LABEL_PREFIX}{}{LABEL_SUFFIX}",
        fit(name, width - frame)
    ))
}

/// Where the daemon label starts on `row`, `after` cells in.
fn label_area(row: Rect, after: u16) -> Rect {
    let x = row
        .x
        .saturating_add(if after == 0 { 0 } else { after + GAP })
        .min(row.right());
    Rect::new(x, row.y, row.right().saturating_sub(x), 1)
}

/// The cells the painted label covers.
fn label_hit(row: Rect, after: u16, store: &BoardStore) -> Option<Rect> {
    let area = label_area(row, after);
    let text = label(store, usize::from(area.width))?;
    let width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
    Some(Rect { width, ..area })
}

/// Paint the label `after` cells into header row 1.
pub fn paint_label(row: Rect, after: u16, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    let area = label_area(row, after);
    let Some(text) = label(store, usize::from(area.width)) else {
        return;
    };
    let style = Style::default().fg(theme.fg).add_modifier(Modifier::BOLD);
    buf.set_stringn(area.x, area.y, text, usize::from(area.width), style);
}

fn entry_text(name: &str, saved: bool, connected: bool) -> String {
    match (connected, saved) {
        (true, _) => format!("{name} (connected)"),
        (false, true) => format!("{name} (saved)"),
        (false, false) => format!("{name} (local)"),
    }
}

/// The open switcher's panel and the text of its rows (entries, or the
/// listing error wrapped to the panel).
fn panel(row: Rect, after: u16, store: &BoardStore) -> Option<(Rect, Vec<String>)> {
    store.daemon_switcher()?;
    let connected = store.connected_daemon();
    let lines: Vec<String> = match store.daemon_switcher_error() {
        Some(error) => vec![error.to_owned()],
        None => store
            .daemons()
            .iter()
            .map(|d| entry_text(&d.name, d.saved, Some(d.name.as_str()) == connected))
            .collect(),
    };
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let wanted = (longest + 4).max(SWITCHER_KEYS.chars().count() + 2);
    let width = u16::try_from(wanted)
        .unwrap_or(u16::MAX)
        .min(row.width.max(1));
    let inner = usize::from(width.saturating_sub(4)).max(1);
    let lines: Vec<String> = match store.daemon_switcher_error() {
        Some(error) => wrap_text(error, inner),
        None => lines.iter().map(|line| fit(line, inner)).collect(),
    };
    // Shift left when the label sits too far right for the panel.
    let x = label_area(row, after)
        .x
        .min(row.right().saturating_sub(width))
        .max(row.x);
    let height = u16::try_from(lines.len() + 2).unwrap_or(u16::MAX);
    Some((Rect::new(x, row.y.saturating_add(1), width, height), lines))
}

/// Paint the open switcher under the daemon label (an overlay).
pub fn paint_dropdown(row: Rect, after: u16, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    let (Some(highlight), Some((rect, lines))) =
        (store.daemon_switcher(), panel(row, after, store))
    else {
        return;
    };
    let rect = rect.intersection(buf.area);
    if rect.width < 3 || rect.height < 3 {
        return;
    }
    let failed = store.daemon_switcher_error().is_some();
    Clear.render(rect, buf);
    Block::bordered()
        .border_style(Style::default().fg(theme.border_focused))
        .title_bottom(fit(SWITCHER_KEYS, usize::from(rect.width - 2)))
        .render(rect, buf);
    let inner = usize::from(rect.width - 2);
    for (index, text) in lines.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        let y = rect.y + 1 + offset;
        if y >= rect.bottom().saturating_sub(1) {
            break;
        }
        let style = if failed {
            Style::default().fg(Color::Red)
        } else if index == highlight {
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
        } else {
            Style::default().fg(theme.fg)
        };
        buf.set_stringn(rect.x + 1, y, format!(" {text:<inner$}"), inner, style);
    }
}

/// Keys: while open the switcher claims every key (`s` opens it, TUI-129).
pub(super) fn handle_key(
    view: &BoardView,
    key: &KeyEvent,
    store: &BoardStore,
) -> Option<EventResult> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // TUI-129: `s` opens it through the menu registry (views/board/keys.rs);
    // without a daemon directory the open shows why.
    store.daemon_switcher()?;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => view.emit(Action::MoveDaemonHighlight(-1)),
        KeyCode::Down | KeyCode::Char('j') => view.emit(Action::MoveDaemonHighlight(1)),
        KeyCode::Enter => view.emit(Action::ChooseDaemon(None)),
        KeyCode::Esc => view.emit(Action::CloseDaemonSwitcher),
        // TUI-121: add / edit / remove saved daemons.
        KeyCode::Char('a') => view.emit(Action::OpenAddDaemonForm),
        KeyCode::Char('e') => view.emit(Action::OpenEditDaemonForm),
        KeyCode::Char('d') | KeyCode::Delete => view.emit(Action::RemoveSavedDaemon),
        _ => {}
    }
    Some(EventResult::consumed())
}

/// TUI-123: left clicks — on the open switcher (a row chooses it, anywhere
/// else closes it) or on the header label (opens it). `after` is the
/// workspace label's width on header `row`.
pub(super) fn handle_mouse(
    view: &BoardView,
    event: &Event,
    store: &BoardStore,
    row: Rect,
    after: u16,
) -> Option<EventResult> {
    let Event::Mouse(mouse) = event else {
        return None;
    };
    if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
        return None;
    }
    let (x, y) = (mouse.column, mouse.row);
    if let Some((rect, _)) = panel(row, after, store) {
        let on_row = rect_contains(rect, x, y) && y > rect.y && y < rect.bottom().saturating_sub(1);
        if on_row && store.daemon_switcher_error().is_none() {
            view.emit(Action::ChooseDaemon(Some(usize::from(y - rect.y - 1))));
        } else if !rect_contains(rect, x, y) {
            view.emit(Action::CloseDaemonSwitcher);
        }
        return Some(EventResult::consumed());
    }
    let hit = label_hit(row, after, store)?;
    if hit.width > 0 && rect_contains(hit, x, y) {
        view.emit(Action::OpenDaemonSwitcher);
        return Some(EventResult::consumed());
    }
    None
}
