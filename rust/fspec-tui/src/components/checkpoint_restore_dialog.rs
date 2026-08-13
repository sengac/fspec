//! RPC-365 — Checkpoint restore modal renderer.
//!
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Thin renderer that paints the `CheckpointsView` restore
//! confirmation/status modal via the shared `dialog_theme::render_dialog`
//! so the `FspecDialog` literal lives in `components/` (RPC-079 keeps all
//! dialog literals out of view modules). The view owns the modal STATE;
//! this function only turns title + body lines into the canonical
//! yellow-accent popup.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};

/// Render the restore modal: a yellow-accent centered popup with the
/// supplied `title` and `body_lines`.
pub fn render_restore_modal(area: Rect, buf: &mut Buffer, title: &str, body_lines: &[String]) {
    let rows: Vec<DialogRow> = body_lines
        .iter()
        .map(|line| DialogRow {
            spans: vec![Span::raw(line.clone())],
            selectable: false,
            selected: false,
        })
        .collect();
    let dialog = FspecDialog {
        accent: Accent::Yellow,
        title,
        rows,
        footer: "",
        min_width: 40,
    };
    render_dialog(area, buf, &dialog);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    fn render(title: &str, body: &[String]) -> String {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("term");
        terminal
            .draw(|f| render_restore_modal(f.area(), f.buffer_mut(), title, body))
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn render_restore_modal_paints_title_and_body() {
        let body = vec![
            "Restore a.txt? This overwrites the working copy.".to_string(),
            String::new(),
            "y: confirm   n: cancel".to_string(),
        ];
        let text = render("Restore Checkpoint", &body);
        assert!(text.contains("Restore Checkpoint"), "missing title");
        assert!(text.contains("a.txt"), "missing body");
        assert!(text.contains("y: confirm"), "missing prompt");
    }
}
