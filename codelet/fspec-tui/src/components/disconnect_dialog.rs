//! Critical-priority Disconnect dialog (RPC-011 CR-1 baseline).
//!
//! Feature: spec/features/disconnect-dialog-cr1-baseline.feature
//! Rules: [1] CR-1 BASELINE: pushed at Priority::Critical when
//!       Action::Disconnected fires; renders the literal strings
//!       'daemon disconnected', 'q to quit', 'r to reconnect'.
//! Rules: [2] CR-1 BASELINE: while topmost, j/k/?/Tab are no-ops;
//!       only 'q' (Action::Quit) and 'r' (Action::ManualReconnect) are
//!       honoured.
//!
//! On Action::Reconnecting(attempt) the dialog body re-renders to
//! "auto-reconnecting (attempt N)…" inline — NO new dialog layer is
//! pushed.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Widget, WidgetRef};
use tui_popup::{Popup, SizedWidgetRef};

use super::{Action, Callback, Component, EventResult, Priority};

/// Stable id used by Compositor::remove on Action::Reconnected.
pub const DISCONNECT_DIALOG_ID: &str = "disconnect-dialog";

/// Critical-priority modal dialog shown when the WebSocketFspecBackend
/// loses its underlying connection.
pub struct DisconnectDialog {
    id: String,
    /// Current attempt count for the auto-reconnect supervisor; `None`
    /// means we have not yet seen an Action::Reconnecting (initial
    /// disconnect state).
    attempt: Option<u32>,
}

impl Default for DisconnectDialog {
    fn default() -> Self {
        Self {
            id: DISCONNECT_DIALOG_ID.to_string(),
            attempt: None,
        }
    }
}

impl DisconnectDialog {
    /// Construct a fresh DisconnectDialog with the canonical id and no
    /// active reconnect attempt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current auto-reconnect attempt counter, if any.
    /// `None` means no Action::Reconnecting has been observed yet.
    pub fn attempt(&self) -> Option<u32> {
        self.attempt
    }

    /// Render the body lines as a single static string. Public for
    /// tests that need to assert content without going through render().
    pub fn body(&self) -> String {
        let header = match self.attempt {
            None => "daemon disconnected".to_string(),
            Some(n) => format!("daemon disconnected — auto-reconnecting (attempt {n})…"),
        };
        format!("{header}\n\nq to quit\nr to reconnect")
    }
}

/// Adapter from `Text<'static>` to the `SizedWidgetRef` trait the
/// `tui_popup::Popup` needs.
#[derive(Debug)]
struct DisconnectBody {
    text: Text<'static>,
    width: u16,
    height: u16,
}

impl WidgetRef for DisconnectBody {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Widget::render(self.text.clone(), area, buf);
    }
}

impl SizedWidgetRef for DisconnectBody {
    fn width(&self) -> usize {
        self.width as usize
    }

    fn height(&self) -> usize {
        self.height as usize
    }
}

impl Component for DisconnectDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    /// CR-1 rule [2]: while topmost, j/k/?/Tab are no-ops. ONLY 'q'
    /// (which emits Action::Quit via the App run loop's main dispatch)
    /// and 'r' (which emits Action::ManualReconnect) are honoured. We
    /// `Consume` every key we handle so it cannot leak to underlying
    /// layers, and we ALSO consume j/k/?/Tab to actively swallow them.
    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => {
                    // 'q' propagates to the App as Action::Quit via the
                    // run loop's KeyEvent → Action mapping. We consume
                    // the key here too so it does not double-trigger.
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                KeyCode::Char('r') => {
                    // Manual reconnect — the App's run loop reads this
                    // KeyEvent independently and emits
                    // Action::ManualReconnect onto the action bus. We
                    // consume here so the keypress is not re-dispatched
                    // to other layers.
                    return EventResult::consumed();
                }
                KeyCode::Char('j')
                | KeyCode::Char('k')
                | KeyCode::Char('?')
                | KeyCode::Tab => {
                    // CR-1 rule [2]: actively swallow navigation keys
                    // so the WorkUnitsListView / HelpDialog / Tab pane
                    // flip cannot fire while we're topmost.
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        EventResult::ignored()
    }

    /// On Action::Reconnecting(n) we update our attempt counter so the
    /// next render() shows "auto-reconnecting (attempt N)…" inline. On
    /// Action::Reconnected the App.dispatch() side removes us from the
    /// compositor — we do not handle Reconnected here.
    fn update(&mut self, action: Action) -> Option<Action> {
        if let Action::Reconnecting(n) = action {
            self.attempt = Some(n);
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let body = self.body();
        let widest = body
            .lines()
            .map(|l| l.chars().count() as u16)
            .max()
            .unwrap_or(0);
        let height = body.lines().count() as u16;
        let sized = DisconnectBody {
            text: Text::raw(body),
            width: widest + 2,
            height,
        };
        let popup = Popup::new(sized).title("Disconnected");
        popup.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    fn render_dialog_80x24(dialog: &mut DisconnectDialog) -> Buffer {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer) -> String {
        let mut all_text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                all_text.push_str(buf[(x, y)].symbol());
            }
            all_text.push('\n');
        }
        all_text
    }

    #[test]
    fn dialog_priority_is_critical() {
        let dialog = DisconnectDialog::new();
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), DISCONNECT_DIALOG_ID);
    }

    #[test]
    fn dialog_renders_required_literal_strings() {
        let mut dialog = DisconnectDialog::new();
        let buf = render_dialog_80x24(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("daemon disconnected"));
        assert!(text.contains("q to quit"));
        assert!(text.contains("r to reconnect"));
    }

    #[test]
    fn reconnecting_action_updates_body_text_inline() {
        let mut dialog = DisconnectDialog::new();
        let _ = dialog.update(Action::Reconnecting(3));
        assert_eq!(dialog.attempt(), Some(3));
        let buf = render_dialog_80x24(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("auto-reconnecting (attempt 3)"));
    }
}
