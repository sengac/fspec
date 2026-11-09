#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

use ratatui::backend::TestBackend;

use ratatui::Terminal;

fn key(code: KeyCode) -> Event {
    Event::Key(crossterm::event::KeyEvent::new(
        code,
        crossterm::event::KeyModifiers::NONE,
    ))
}

mod part_1;
