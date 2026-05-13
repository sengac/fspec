//! BoardView — rich box-drawing Kanban grid + work-unit details strip.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc013-board-footer.feature
//!   - spec/features/rpc014-board-grid.feature
//!   - spec/features/rpc014-source-shape.feature
//! Cards: RPC-012 (placeholder skeleton), RPC-013 (view-aware footer),
//!        RPC-014 (rich grid + details strip).
//!
//! Renders the seven canonical kanban columns with box-drawing
//! separators (`├ ┬ ┼ ┴ ┤`), a 5-row work-unit details strip above
//! the column headers, focused-column highlighting and work-type cell
//! coloring. Per-column viewport scroll (the `⏩`/`🟢` indicators and
//! the `↑`/`↓` scroll arrows) lands in RPC-016.
//!
//! BoardView reads from a `&BoardStore` passed in via `render_with_store`
//! — it does NOT own any work-units state. Keyboard handling emits
//! `Action::EnterWorkUnit` / `Action::OpenAgentView` /
//! `Action::FocusNextColumn` etc. onto the action bus that
//! `App::dispatch` consumes.

use std::sync::Arc;

use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, EventResult};
use crate::store::{BoardStore, COLUMN_ORDER};
use crate::theme::Theme;

pub mod checkpoint_status;
pub mod columns;
pub mod details_strip;
pub mod grid;
pub mod header;
pub mod keybinding_shortcuts;
pub mod logo;

use self::columns::{paint_column_headers, paint_content_rows};
use self::grid::{
    build_border_row, calculate_column_widths, column_width_at, SeparatorType,
};

/// BoardView holds NO work-units state — only the action bus + theme.
pub struct BoardView {
    pub theme: Arc<Theme>,
    pub action_tx: Option<UnboundedSender<Action>>,
}

impl BoardView {
    pub fn new(theme: Arc<Theme>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            theme,
            action_tx: Some(action_tx),
        }
    }

    fn emit(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Handle a keyboard event against the supplied store snapshot.
    /// The store is &-borrow only; mutation flows through App::dispatch
    /// in response to the emitted action.
    pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };

        // Shift+Right → open AgentView (with or without an attached session).
        if key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT) {
            let target = self.selected_session(store);
            self.emit(Action::OpenAgentView(target));
            return EventResult::consumed();
        }

        // Enter → hand off to AgentView for the focused work unit.
        if key.code == KeyCode::Enter {
            if let Some(unit) = store.selected_work_unit() {
                self.emit(Action::EnterWorkUnit(unit.id.clone()));
                return EventResult::consumed();
            }
            return EventResult::ignored();
        }

        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.emit(Action::FocusPrevColumn);
                return EventResult::consumed();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.emit(Action::FocusNextColumn);
                return EventResult::consumed();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.emit(Action::SelectNext);
                return EventResult::consumed();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.emit(Action::SelectPrev);
                return EventResult::consumed();
            }
            KeyCode::Char('[') => {
                self.emit(Action::ReorderUp);
                return EventResult::consumed();
            }
            KeyCode::Char(']') => {
                self.emit(Action::ReorderDown);
                return EventResult::consumed();
            }
            _ => {}
        }

        EventResult::ignored()
    }

    fn selected_session(&self, store: &BoardStore) -> Option<SessionId> {
        let unit = store.selected_work_unit()?;
        store.session_for(&unit.id).cloned()
    }

    /// Render the rich BoardView against the supplied store. Composes
    /// the box-drawing topology row-by-row: top border, 4-row header
    /// strip (RPC-015 logo + checkpoints + keybindings), ├──┤ plain
    /// separator, 5-row details strip, ├┬┤ separator, column header
    /// row, ├┼┤ separator, content rows, ├┴┤ separator, RPC-013 footer
    /// string, bottom border.
    pub fn render_with_store(&self, area: Rect, buf: &mut Buffer, store: &BoardStore) {
        if area.width < 4 || area.height < 17 {
            return;
        }
        let widths = calculate_column_widths(area.width);
        let inner_width: u16 = (0..COLUMN_ORDER.len() as u16)
            .map(|i| column_width_at(i as usize, widths))
            .sum::<u16>()
            + (COLUMN_ORDER.len() as u16 - 1);
        if inner_width + 2 > area.width {
            return;
        }
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // top border
                Constraint::Length(4), // RPC-015 header strip
                Constraint::Length(1), // ├──┤ plain separator (RPC-015)
                Constraint::Length(5), // details strip (RPC-014)
                Constraint::Length(1), // ├┬┤ separator
                Constraint::Length(1), // column header
                Constraint::Length(1), // ├┼┤ separator
                Constraint::Min(0),    // content
                Constraint::Length(1), // ├┴┤ separator
                Constraint::Length(1), // footer
                Constraint::Length(1), // bottom border
            ])
            .split(area);
        let border_style = Style::default().fg(self.theme.border);

        paint_border_string(
            split[0], buf,
            &build_border_row(widths, "┌", "┐", SeparatorType::Plain), border_style,
        );
        paint_side_borders(split[1], buf, border_style);
        header::paint(inner_rect(split[1]), buf, store, &self.theme);
        paint_border_string(
            split[2], buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Plain), border_style,
        );
        paint_side_borders(split[3], buf, border_style);
        details_strip::render(inner_rect(split[3]), buf, store.selected_work_unit());
        paint_border_string(
            split[4], buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Top), border_style,
        );
        paint_side_borders(split[5], buf, border_style);
        paint_column_headers(split[5], buf, widths, store, &self.theme);
        paint_border_string(
            split[6], buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Cross), border_style,
        );
        paint_content_rows(split[7], buf, widths, store, &self.theme);
        paint_border_string(
            split[8], buf,
            &build_border_row(widths, "├", "┤", SeparatorType::Bottom), border_style,
        );
        paint_side_borders(split[9], buf, border_style);
        render_footer(inner_rect(split[9]), buf, &self.theme);
        paint_border_string(
            split[10], buf,
            &build_border_row(widths, "└", "┘", SeparatorType::Plain), border_style,
        );
    }
}

fn paint_border_string(area: Rect, buf: &mut Buffer, body: &str, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    Paragraph::new(Line::from(Span::styled(body.to_string(), style)))
        .render(area, buf);
}

fn paint_side_borders(area: Rect, buf: &mut Buffer, style: Style) {
    if area.width < 2 || area.height == 0 {
        return;
    }
    for y in 0..area.height {
        buf.set_string(area.x, area.y + y, "│", style);
        buf.set_string(area.x + area.width - 1, area.y + y, "│", style);
    }
}

fn inner_rect(area: Rect) -> Rect {
    if area.width < 2 {
        return area;
    }
    Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width - 2,
        height: area.height,
    }
}

/// RPC-013: paint the 1-row Board footer string. Literal port of
/// `src/tui/components/UnifiedBoardLayout.tsx:504-511`.
fn render_footer(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let dim = Style::default().fg(theme.dim);
    let key = Style::default().fg(theme.fg).add_modifier(Modifier::BOLD);
    let line = Line::from(vec![
        Span::styled("← → ", key),
        Span::styled("Columns ", dim),
        Span::styled("◆ ", dim),
        Span::styled("↑↓ ", key),
        Span::styled("Work Units ", dim),
        Span::styled("◆ ", dim),
        Span::styled("[ ", key),
        Span::styled("Priority Up ", dim),
        Span::styled("◆ ", dim),
        Span::styled("] ", key),
        Span::styled("Priority Down ", dim),
        Span::styled("◆ ", dim),
        Span::styled("↵ ", key),
        Span::styled("Work Agent ", dim),
        Span::styled("◆ ", dim),
        Span::styled("ESC ", key),
        Span::styled("Back", dim),
    ]);
    Paragraph::new(line).render(area, buf);
}
