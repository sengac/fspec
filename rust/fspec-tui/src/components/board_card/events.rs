//! BOARD-024 — modal event routing and Action fan-out for the card dialog.
//!
//! Every key, paste and mouse event is consumed while the dialog is open so
//! board navigation, app shortcuts, agent input and mux focus never change
//! underneath it.

use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::page::Target;
use super::{BoardCardDialog, BoardCardOrigin, Mode, Status, BOARD_CARD_DIALOG_ID};
use crate::components::{Action, Callback, Component, EventResult, Priority};

const WHEEL_ROWS: usize = 3;

fn remove_callback() -> Callback {
    Box::new(|compositor| {
        let _ = compositor.remove(BOARD_CARD_DIALOG_ID);
    })
}

impl BoardCardDialog {
    fn chooser_event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::consumed();
        };
        match key.code {
            KeyCode::Esc => return EventResult::Consumed(Some(remove_callback())),
            KeyCode::Up | KeyCode::BackTab => self.chooser_selected = 0,
            KeyCode::Down | KeyCode::Tab => self.chooser_selected = 1,
            KeyCode::Enter if self.chooser_selected == 0 => {
                let id = self.card_id.clone();
                self.emit(match self.origin {
                    BoardCardOrigin::Board => Action::EnterWorkUnit(id),
                    BoardCardOrigin::Mux => Action::MuxEnterWorkUnit(id),
                });
                return EventResult::Consumed(Some(remove_callback()));
            }
            KeyCode::Enter => {
                self.mode = Mode::Details;
                self.pages.clear();
                self.request(self.card_id.clone());
            }
            _ => {}
        }
        EventResult::consumed()
    }

    fn details_event(&mut self, event: &Event) -> EventResult {
        let target = match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.back();
                    None
                }
                // TUI-130: y copies the card id (c copies the page, through
                // `copy_text`).
                KeyCode::Char('y') => {
                    if let Some(page) = self.pages.last() {
                        self.emit(Action::CopyToClipboard(page.id.clone()));
                    }
                    None
                }
                code => self.pages.last_mut().and_then(|page| page.key(code)),
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    self.scroll_page(WHEEL_ROWS as isize);
                    None
                }
                MouseEventKind::ScrollUp => {
                    self.scroll_page(-(WHEEL_ROWS as isize));
                    None
                }
                MouseEventKind::Down(MouseButton::Left) => self
                    .pages
                    .last_mut()
                    .and_then(|page| page.click(mouse.column, mouse.row)),
                _ => None,
            },
            _ => None,
        };
        if let Some(target) = target {
            self.activate(target);
        }
        EventResult::consumed()
    }

    fn scroll_page(&mut self, delta: isize) {
        if let Some(page) = self.pages.last_mut() {
            page.scroll_by(delta);
        }
    }

    fn activate(&mut self, target: Target) {
        match target {
            Target::Link(id) => self.request(id),
            Target::Attachment(path) => {
                self.status = None;
                self.emit(Action::OpenAttachment(path));
            }
        }
    }
}

impl Component for BoardCardDialog {
    fn priority(&self) -> Priority {
        Priority::Foreground
    }

    fn id(&self) -> &str {
        BOARD_CARD_DIALOG_ID
    }

    /// TUI-130: c and Ctrl+C copy the open details page's whole text.
    fn copy_text(&self) -> Option<String> {
        match self.mode {
            Mode::Details => self.pages.last().map(super::page::Page::plain_text),
            Mode::Chooser => None,
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match self.mode {
            Mode::Chooser => self.chooser_event(event),
            Mode::Details => self.details_event(event),
        }
    }

    fn update(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::BoardCardDetailsLoaded {
                request_generation,
                work_unit_id,
                result,
            } => self.complete(request_generation, &work_unit_id, &result),
            Action::BoardCardAttachmentFailed { path, message }
                if matches!(self.mode, Mode::Details) && !self.pages.is_empty() =>
            {
                self.status = Some(Status::Error(format!("Error: {path}: {message}")));
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self.mode {
            Mode::Chooser => self.render_chooser(area, buf),
            Mode::Details => self.render_details(area, buf),
        }
    }
}
