//! RPC-445: the board header's workspace selector — the row-1 label,
//! the dropdown overlay beneath it, and its key / mouse handling.
//!
//! Feature: spec/features/board-workspace-dropdown.feature
//!
//! `w` (modifier-free) or a click on header row 1 opens the dropdown;
//! while open it is modal: Up/Down (k/j) move, Enter chooses, Esc closes,
//! a click on an entry chooses it and a click elsewhere closes it.
//! TUI-122: `a` adds a workspace by path, `d` removes the highlighted one.
//! TUI-123: the daemon label and the open daemon switcher take clicks too
//! (`daemon_switcher::handle_mouse`).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, Widget};

use crate::components::{Action, EventResult};
use crate::mouse::rect_contains;
use crate::store::BoardStore;
use crate::theme::Theme;

use super::BoardView;

const LABEL_PREFIX: &str = "Workspace: ";
/// TUI-122: the dropdown's add / remove keys, on its bottom border.
const DROPDOWN_KEYS: &str = " a add · d remove ";
/// TUI-127: with no workspaces listed there is nothing to remove.
const EMPTY_DROPDOWN_KEYS: &str = " a add ";
/// TUI-127: the one row of an empty dropdown.
const NO_WORKSPACES: &str = "No workspaces";

/// The key hint for the dropdown's bottom border.
fn dropdown_keys(store: &BoardStore) -> &'static str {
    if store.workspaces().is_empty() {
        EMPTY_DROPDOWN_KEYS
    } else {
        DROPDOWN_KEYS
    }
}

/// The row-1 label for the selected workspace, if the list has loaded.
pub fn label(store: &BoardStore) -> Option<String> {
    store
        .selected_workspace()
        .map(|entry| format!("{LABEL_PREFIX}{} ▾", entry.name))
}

/// Paint the row-1 label into `row`.
pub fn paint_label(row: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    let width = label_width(store);
    if let Some(text) = label(store) {
        let style = Style::default().fg(theme.fg).add_modifier(Modifier::BOLD);
        buf.set_stringn(row.x, row.y, text, usize::from(row.width), style);
    }
    // RPC-448: the daemon label follows the workspace label.
    super::daemon_switcher::paint_label(row, width, buf, store, theme);
}

/// Cells the workspace label occupies (0 before the list loads).
fn label_width(store: &BoardStore) -> u16 {
    label(store).map_or(0, |text| {
        u16::try_from(text.chars().count()).unwrap_or(u16::MAX)
    })
}

/// The dropdown panel anchored under `row`: one line per workspace
/// inside a 1-cell border, as wide as the longest name. TUI-127: an empty
/// list has the one "No workspaces" row.
pub fn panel_rect(row: Rect, store: &BoardStore) -> Rect {
    let entries = store.workspaces();
    let longest = entries
        .iter()
        .map(|entry| entry_text(&entry.name, entry.start).chars().count())
        .max()
        .unwrap_or(NO_WORKSPACES.chars().count());
    // TUI-122: wide enough for the key hint on the bottom border.
    let width = u16::try_from((longest + 4).max(dropdown_keys(store).chars().count() + 2))
        .unwrap_or(u16::MAX)
        .min(row.width.max(1));
    let height = u16::try_from(entries.len().max(1) + 2).unwrap_or(u16::MAX);
    Rect::new(row.x, row.y.saturating_add(1), width, height)
}

fn entry_text(name: &str, start: bool) -> String {
    if start {
        format!("{name} (start)")
    } else {
        name.to_owned()
    }
}

/// Paint the open dropdown (an overlay; call after the board body).
pub fn paint_dropdown(row: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    super::daemon_switcher::paint_dropdown(row, label_width(store), buf, store, theme);
    let Some(highlight) = store.workspace_dropdown() else {
        return;
    };
    let panel = panel_rect(row, store).intersection(buf.area);
    if panel.width < 3 || panel.height < 3 {
        return;
    }
    Clear.render(panel, buf);
    Block::bordered()
        .border_style(Style::default().fg(theme.border_focused))
        .title_bottom(dropdown_keys(store))
        .render(panel, buf);
    let inner_width = usize::from(panel.width - 2);
    if store.workspaces().is_empty() {
        let text = format!(" {NO_WORKSPACES}");
        let style = Style::default().fg(theme.dim);
        buf.set_stringn(panel.x + 1, panel.y + 1, text, inner_width, style);
        return;
    }
    for (index, entry) in store.workspaces().iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        let y = panel.y + 1 + offset;
        if y >= panel.bottom().saturating_sub(1) {
            break;
        }
        let style = if index == highlight {
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
        } else {
            Style::default().fg(theme.fg)
        };
        let text = format!(" {:<inner_width$}", entry_text(&entry.name, entry.start));
        buf.set_stringn(panel.x + 1, y, text, inner_width, style);
    }
}

/// Keys: while open the dropdown claims every key (`w` opens it, TUI-129).
pub(super) fn handle_key(
    view: &BoardView,
    key: &KeyEvent,
    store: &BoardStore,
) -> Option<EventResult> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    // RPC-448: the daemon switcher's `s` and its open dropdown come first.
    if store.workspace_dropdown().is_none() {
        if let Some(result) = super::daemon_switcher::handle_key(view, key, store) {
            return Some(result);
        }
    }
    // TUI-129: `w` opens it through the menu registry (views/board/keys.rs).
    store.workspace_dropdown()?;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => view.emit(Action::MoveWorkspaceHighlight(-1)),
        KeyCode::Down | KeyCode::Char('j') => view.emit(Action::MoveWorkspaceHighlight(1)),
        KeyCode::Enter => view.emit(Action::ChooseWorkspace(None)),
        KeyCode::Esc => view.emit(Action::CloseWorkspaceDropdown),
        // TUI-122: add a workspace by path / remove the highlighted one.
        KeyCode::Char('a') => view.emit(Action::OpenAddWorkspaceDialog),
        KeyCode::Char('d') | KeyCode::Delete => view.emit(Action::RemoveWorkspace),
        _ => {}
    }
    Some(EventResult::consumed())
}

/// Left clicks on header row 1 and on the open dropdown.
pub(super) fn handle_mouse(
    view: &BoardView,
    event: &Event,
    store: &BoardStore,
) -> Option<EventResult> {
    let Event::Mouse(mouse) = event else {
        return None;
    };
    if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
        return None;
    }
    let row = view.last_workspace_row.get()?;
    let (x, y) = (mouse.column, mouse.row);
    if store.workspace_dropdown().is_some() {
        let panel = panel_rect(row, store);
        let inside_entries =
            rect_contains(panel, x, y) && y > panel.y && y < panel.bottom().saturating_sub(1);
        if inside_entries {
            view.emit(Action::ChooseWorkspace(Some(usize::from(y - panel.y - 1))));
        } else {
            view.emit(Action::CloseWorkspaceDropdown);
        }
        return Some(EventResult::consumed());
    }
    // TUI-123: the open daemon switcher and the daemon label.
    if let Some(result) =
        super::daemon_switcher::handle_mouse(view, event, store, row, label_width(store))
    {
        return Some(result);
    }
    let label_width = label(store).map(|text| text.chars().count()).unwrap_or(0);
    let hit = Rect {
        width: u16::try_from(label_width)
            .unwrap_or(u16::MAX)
            .min(row.width),
        ..row
    };
    if label_width > 0 && rect_contains(hit, x, y) {
        view.emit(Action::OpenWorkspaceDropdown);
        return Some(EventResult::consumed());
    }
    None
}
