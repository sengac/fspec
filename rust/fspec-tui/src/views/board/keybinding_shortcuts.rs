//! Header keybinding hint row — Rust port of
//! `src/tui/components/KeybindingShortcuts.tsx`.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015 (superseded by BOARD-023 for the row content).
//!
//! BOARD-023 (R10): the six-action chord
//!   `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux`
//! was collapsed into the `BoardKeybindingDialog` ('u' key, titled
//! 'Actions'). This row now paints a single short hint from the single
//! source of truth `board_shortcuts::CHORD_HINT` — so the hint and the
//! dialog's rows can never drift apart.
//!
//! The `.` New Agent binding is wired in RPC-395 (opens AgentView),
//! the `/` Search binding in BOARD-022 (work-unit search dialog), and
//! the `M` Mux binding in MUX-009 (opens the MuxConfigDialog — the
//! same dialog bare `/mux` opens). All six now open from the
//! actions dialog instead of being spelled out here.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::Theme;

use super::board_shortcuts::CHORD_HINT;

/// Render the 1-row keybinding hint into `area`. `area.height` must
/// be at least 1.
pub fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // TS source (`KeybindingShortcuts.tsx`) renders a single plain
    // `<Text>` with no color/bold attributes — so port it as one span
    // styled with the theme's primary fg.
    let style = Style::default().fg(theme.fg);
    let line = Line::from(Span::styled(CHORD_HINT.to_string(), style));
    Paragraph::new(line).render(
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
        buf,
    );
}
