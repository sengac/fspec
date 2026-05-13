//! Critical-priority Help dialog (RPC-008 rule [15]).
//!
//! Feature: spec/features/fspec-tui-help-dialog.feature
//!
//! Triggered by the `?` key at App-level (NOT inside HelloComponent —
//! the App layer pushes this onto the compositor). Body lists exactly
//! the `?`, ESC, and `q` keybindings. ESC returns
//! `EventResult::Consumed(Some(callback))` where the callback removes
//! the dialog by id.
//!
//! Production rendering uses the `tui_popup::Popup` adapter (per
//! RPC-002 Q5) — a hand-rolled `centered_rect` helper would be the
//! fallback. The `tui-popup` adapter handles centering + bordering for
//! us so we don't reinvent that code path.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Widget, WidgetRef};
use tui_popup::{Popup, SizedWidgetRef};

use super::{Callback, Component, EventResult, Priority};

const HELP_BODY: &str = "j/k     Navigate\nTab     Switch pane\n?       Toggle this help\nq       Quit fspec-tui\nEnter   Send\nCtrl+C  Interrupt\nESC     Dismiss this dialog";

/// Critical-priority modal dialog listing the App-level keybindings.
pub struct HelpDialog {
    id: String,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self {
            id: "help-dialog".to_string(),
        }
    }
}

impl HelpDialog {
    /// Construct a HelpDialog with the canonical id `"help-dialog"`.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Adapter from `Text<'static>` to the `SizedWidgetRef` trait the
/// `tui_popup::Popup` needs. Holds an owned Text plus the (width,
/// height) the popup should size itself to. Constructed inline by
/// `HelpDialog::render`.
#[derive(Debug)]
struct HelpBody {
    text: Text<'static>,
    width: u16,
    height: u16,
}

impl WidgetRef for HelpBody {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Widget::render(self.text.clone(), area, buf);
    }
}

impl SizedWidgetRef for HelpBody {
    fn width(&self) -> usize {
        self.width as usize
    }

    fn height(&self) -> usize {
        self.height as usize
    }
}

impl Component for HelpDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Esc {
                let id = self.id.clone();
                let callback: Callback = Box::new(move |compositor| {
                    let _ = compositor.remove(&id);
                });
                return EventResult::Consumed(Some(callback));
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let body = Text::raw(HELP_BODY);
        // Width: the longest line plus a 2-cell breathing margin so the
        // border doesn't cuddle the text.
        let widest = HELP_BODY
            .lines()
            .map(|l| l.chars().count() as u16)
            .max()
            .unwrap_or(0);
        let height = HELP_BODY.lines().count() as u16;
        let sized = HelpBody {
            text: body,
            width: widest + 2,
            height,
        };
        let popup = Popup::new(sized).title("Help");
        popup.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    /// Helper: render an isolated HelpDialog onto an 80x24 TestBackend
    /// buffer and return the resulting [`Buffer`] for assertion. Shared
    /// between the keybinding-content scenario and the snapshot scenario.
    fn render_help_dialog_80x24() -> Buffer {
        let mut dialog = HelpDialog::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    /// Scenario: HelpDialog renders via the tui-popup adapter at
    /// Priority::Critical
    #[test]
    fn help_dialog_renders_via_tui_popup_adapter_at_priority_critical() {
        // @step Given an isolated HelpDialog component with id "help-dialog"
        let dialog = HelpDialog::new();
        assert_eq!(dialog.id(), "help-dialog");

        // @step When I inspect its priority()
        // @step Then it returns Priority::Critical
        assert_eq!(dialog.priority(), Priority::Critical);

        // @step When I inspect its render(...) implementation
        // @step Then it constructs a `tui_popup::Popup` wrapping a `SizedWidgetRef` adapter
        // (Compile-time: `HelpBody: SizedWidgetRef` and the body of
        // `HelpDialog::render` constructs `Popup::new(sized).title(...)`.
        // We assert the source-shape via a sibling source_shape_trait
        // test in a later push if needed.)
        let src = include_str!("help_dialog.rs");
        assert!(
            src.contains("Popup::new("),
            "HelpDialog::render must construct tui_popup::Popup"
        );
        assert!(
            src.contains("SizedWidgetRef"),
            "HelpDialog::render must use a SizedWidgetRef adapter"
        );

        // @step And it does NOT use a hand-rolled `centered_rect` helper as the production code path
        // We look for a `fn ` definition of the helper, not the
        // string itself (avoids matching the assertion message above).
        let needle = ["fn ", "centered", "_rect"].concat();
        assert!(
            !src.contains(&needle),
            "HelpDialog::render must NOT define a hand-rolled centering helper as the production code path"
        );
    }

    /// Scenario: HelpDialog static body lists exactly the '?', ESC, and
    /// 'q' keybindings
    #[test]
    fn help_dialog_static_body_lists_question_esc_and_q_keybindings() {
        // @step Given an isolated HelpDialog component
        // @step When I render it onto an 80x24 TestBackend buffer
        let buf = render_help_dialog_80x24();
        let mut all_text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                all_text.push_str(buf[(x, y)].symbol());
            }
            all_text.push('\n');
        }

        // @step Then the buffer contains a line including "?"
        assert!(
            all_text.contains('?'),
            "rendered buffer must contain '?'. Got:\n{all_text}"
        );

        // @step And the buffer contains a line including "ESC"
        assert!(
            all_text.contains("ESC"),
            "rendered buffer must contain 'ESC'. Got:\n{all_text}"
        );

        // @step And the buffer contains a line including "q"
        assert!(
            all_text.contains('q'),
            "rendered buffer must contain 'q'. Got:\n{all_text}"
        );
    }

    /// Scenario: HelpDialog rendering is byte-equal across runs (insta
    /// snapshot) — captures the cell grid as a Vec<String> and asserts
    /// it via `insta::assert_yaml_snapshot!`.
    #[test]
    fn help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        // @step Given an isolated HelpDialog component
        // @step When I render it onto an 80x24 TestBackend buffer
        let buf = render_help_dialog_80x24();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }

        // @step And I serialise the buffer cell grid via `insta::assert_yaml_snapshot!`
        // @step Then the serialised output matches the snapshot file "help_dialog__centered_popup_80x24.snap"
        insta::assert_yaml_snapshot!("help_dialog__centered_popup_80x24", rows);
    }

    /// Scenario (RPC-009 fspec-tui-help-dialog-body-rpc009.feature):
    /// HelpDialog body lists every keybinding from the RPC-009 scope on
    /// one line each.
    #[test]
    fn help_dialog_body_lists_every_keybinding_from_the_rpc009_scope() {
        // @step Given an isolated HelpDialog component
        // @step When the dialog is rendered onto an 80x24 TestBackend
        let buf = render_help_dialog_80x24();
        let mut all_text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                all_text.push_str(buf[(x, y)].symbol());
            }
            all_text.push('\n');
        }
        // @step Then the rendered buffer contains the substring "j"
        assert!(all_text.contains('j'), "rendered buffer must contain 'j'");
        // @step And the rendered buffer contains the substring "k"
        assert!(all_text.contains('k'), "rendered buffer must contain 'k'");
        // @step And the rendered buffer contains the substring "Tab"
        assert!(all_text.contains("Tab"), "rendered buffer must contain 'Tab'");
        // @step And the rendered buffer contains the substring "?"
        assert!(all_text.contains('?'), "rendered buffer must contain '?'");
        // @step And the rendered buffer contains the substring "q"
        assert!(all_text.contains('q'), "rendered buffer must contain 'q'");
        // @step And the rendered buffer contains the substring "Enter"
        assert!(all_text.contains("Enter"), "rendered buffer must contain 'Enter'");
        // @step And the rendered buffer contains the substring "Ctrl+C"
        assert!(all_text.contains("Ctrl+C"), "rendered buffer must contain 'Ctrl+C'");
        // @step And the rendered buffer contains the substring "ESC"
        assert!(all_text.contains("ESC"), "rendered buffer must contain 'ESC'");
    }

    /// Scenario (RPC-009): HelpDialog still uses the tui-popup adapter at
    /// Priority::Critical (RPC-008 invariant).
    #[test]
    fn help_dialog_still_uses_tui_popup_adapter_at_priority_critical_rpc009_invariant() {
        // @step Given an isolated HelpDialog component
        let dialog = HelpDialog::new();
        // @step Then its priority() returns Priority::Critical
        assert_eq!(dialog.priority(), Priority::Critical);
        // @step And its render(...) implementation constructs a `tui_popup::Popup` wrapping a `SizedWidgetRef` adapter
        let src = include_str!("help_dialog.rs");
        assert!(src.contains("Popup::new("));
        assert!(src.contains("SizedWidgetRef"));
        // @step And it does NOT define a hand-rolled `centered_rect` helper as the production code path
        let needle = ["fn ", "centered", "_rect"].concat();
        assert!(!src.contains(&needle));
    }

    /// Scenario (RPC-009): HelpDialog rendering is byte-equal across runs (snapshot regenerated)
    #[test]
    fn help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot_rpc009() {
        // @step Given an isolated HelpDialog component rendered onto an 80x24 TestBackend
        let buf = render_help_dialog_80x24();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        // @step When the buffer cell grid is serialised via `insta::assert_yaml_snapshot!`
        // @step Then the serialised output matches the regenerated snapshot file "help_dialog__centered_popup_80x24.snap"
        insta::assert_yaml_snapshot!("help_dialog__centered_popup_80x24", rows);
    }
}
