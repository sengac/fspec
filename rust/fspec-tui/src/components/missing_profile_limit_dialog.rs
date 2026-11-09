//! BUG-547 — Priority::Foreground prompt for a local profile model's missing
//! context window or max output.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature
//!
//! Opened when a model selection or session creation is refused with
//! `codelet_sessions::profile_limits::MissingProfileLimit`. A single-line
//! number input (like `AddWorkspaceDialog`): Enter with a whole positive
//! token count emits `Action::SaveMissingProfileLimit` (the App saves it to
//! the profile and retries), Esc emits `Action::MissingProfileLimitCancelled`
//! (the App shows the refusal). Nothing is filled in for the user.

use codelet_sessions::profile_limits::MissingProfileLimit;
use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, TextArea};

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR};
use super::{Action, Callback, Component, EventResult, Priority};

/// Compositor id of the missing-limit prompt.
pub const MISSING_PROFILE_LIMIT_DIALOG_ID: &str = "missing-profile-limit-dialog";

/// What to run again once the missing limit is saved.
#[derive(Debug, Clone)]
pub enum MissingLimitRetry {
    /// A model selection (`Action::ModelSelected`). BUG-576: `selection`
    /// numbers it among the session's selections, so it is selected again
    /// only when no later selection was made.
    ModelSelected {
        session: Option<codelet_rpc_types::SessionId>,
        provider: String,
        model: String,
        selection: u64,
    },
    /// A session creation (`run_session_creation`), seeding the active
    /// session when `seed_active` is set.
    SessionCreation {
        isolated: bool,
        role: Option<String>,
        workspace: Option<String>,
        card: Option<String>,
        seed_active: bool,
        originating_session: Option<codelet_rpc_types::SessionId>,
    },
}

/// The number input for one missing limit.
pub struct MissingProfileLimitDialog {
    limit: MissingProfileLimit,
    retry: MissingLimitRetry,
    textarea: TextArea<'static>,
    error: Option<String>,
    action_tx: UnboundedSender<Action>,
}

impl MissingProfileLimitDialog {
    pub fn new(
        limit: MissingProfileLimit,
        retry: MissingLimitRetry,
        action_tx: UnboundedSender<Action>,
    ) -> Self {
        let mut textarea = TextArea::new(vec![String::new()]);
        textarea.set_cursor_line_style(Style::default());
        Self {
            limit,
            retry,
            textarea,
            error: None,
            action_tx,
        }
    }

    /// The typed value.
    pub fn draft(&self) -> String {
        self.textarea.lines().join("")
    }

    fn close() -> Callback {
        Box::new(|compositor| {
            let _ = compositor.remove(MISSING_PROFILE_LIMIT_DIALOG_ID);
        })
    }

    fn send(&self, action: Action) {
        if self.action_tx.send(action).is_err() {
            tracing::error!(
                "missing-limit prompt: action bus closed; the answer was not delivered"
            );
        }
    }

    fn submit(&mut self) -> EventResult {
        match self.draft().trim().parse::<u32>() {
            Ok(value) if value > 0 => {
                self.send(Action::SaveMissingProfileLimit {
                    limit: self.limit.clone(),
                    value,
                    retry: self.retry.clone(),
                });
                EventResult::Consumed(Some(Self::close()))
            }
            _ => {
                self.error = Some("Enter a whole number of tokens greater than 0".to_owned());
                EventResult::consumed()
            }
        }
    }

    fn cancel(&self) -> EventResult {
        self.send(Action::MissingProfileLimitCancelled {
            limit: self.limit.clone(),
            retry: self.retry.clone(),
        });
        EventResult::Consumed(Some(Self::close()))
    }
}

impl Component for MissingProfileLimitDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        MISSING_PROFILE_LIMIT_DIALOG_ID
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => self.cancel(),
                KeyCode::Enter => self.submit(),
                KeyCode::Char(c) if !c.is_ascii_digit() => EventResult::consumed(),
                _ => {
                    let _ = self
                        .textarea
                        .input(Input::from(crossterm::event::KeyEvent::new(
                            key.code,
                            key.modifiers,
                        )));
                    EventResult::consumed()
                }
            },
            Event::Paste(text) => {
                let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                let _ = self.textarea.insert_str(digits);
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let draft = self.draft();
        let mut rows = vec![
            DialogRow {
                spans: vec![Span::raw(format!(
                    "{} has no {}.",
                    self.limit.model,
                    self.limit.field.description()
                ))],
                selectable: false,
                selected: false,
            },
            DialogRow {
                spans: vec![Span::raw(format!(
                    "It is saved as {} on the {} profile.",
                    self.limit.field.config_key(),
                    self.limit.profile
                ))],
                selectable: false,
                selected: false,
            },
            DialogRow {
                spans: vec![Span::raw(format!(
                    "{}: {}",
                    self.limit.field.label(),
                    if draft.is_empty() {
                        " "
                    } else {
                        draft.as_str()
                    }
                ))],
                selectable: false,
                selected: false,
            },
        ];
        if let Some(error) = &self.error {
            rows.push(DialogRow {
                spans: vec![Span::styled(error.clone(), Style::default().fg(Color::Red))],
                selectable: false,
                selected: false,
            });
        }
        let sep = FOOTER_SEPARATOR;
        let footer = format!("Enter Save{sep}Esc Cancel");
        let title = format!("{} needed", self.limit.field.label());
        render_dialog(
            area,
            buf,
            &FspecDialog {
                accent: Accent::Cyan,
                title: title.as_str(),
                rows,
                footer: footer.as_str(),
                min_width: 60,
                query_row: None,
            },
        );
    }
}
