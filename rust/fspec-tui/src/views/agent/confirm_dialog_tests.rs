//! Unit tests for `confirm_dialog.rs` (moved out to keep the widget file
//! under the 300-LoC source-shape ceiling, TUI-124).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn confirm_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
    let dialog = ConfirmDialog::new(
        "Delete Session",
        "This action cannot be undone.",
        "Delete",
        None,
        "Cancel",
    );
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            dialog.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    let buf = terminal.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::with_capacity(buf.area.width as usize);
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    insta::assert_yaml_snapshot!("confirm_dialog__centered_popup_80x24", rows);
}
