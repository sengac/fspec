//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{buffer::Buffer, layout::Rect};
use unicode_width::UnicodeWidthStr;

pub use super::selection::selected_position;
use super::*;

pub fn summary(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(),
        title: format!("Summary {id}"),
        status: "backlog".into(),
        work_type: "story".into(),
        description: Some("TRUNCATED BOARD SUMMARY".into()),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

pub fn app_with(backend: Arc<dyn FspecBackend>) -> App {
    let mut app = App::new(backend);
    app.dispatch(Action::WorkUnitsLoaded(vec![
        summary("AUTH-001"),
        summary("AUTH-002"),
    ]));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    app
}

pub fn assert_mux_layout_preserved(
    current: &codelet_fspec_tui::MuxConfig,
    original: &codelet_fspec_tui::MuxConfig,
) {
    assert_eq!(current.orientation, original.orientation);
    assert_eq!(current.splits, original.splits);
    assert_eq!(current.panes, original.panes);
    assert_eq!(current.enabled, original.enabled);
}

pub fn flush(app: &mut App) {
    for _ in 0..128 {
        match app.try_recv_action() {
            Some(action) => app.dispatch(action),
            None => return,
        }
    }
    assert!(app.try_recv_action().is_none(), "action bus did not settle");
}

pub fn event(app: &mut App, event: Event) -> bool {
    let consumed = app.handle_event(&event).is_consumed();
    flush(app);
    consumed
}

pub fn press(app: &mut App, code: KeyCode) -> bool {
    event(app, Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}

pub async fn settle(app: &mut App) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        for _ in 0..128 {
            flush(app);
            let Some(task) = app.next_pending_task() else {
                return Ok(());
            };
            task.await?;
        }
        Err("pending tasks did not settle".into())
    })
    .await?
}

pub fn buffer(app: &mut App, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
}

pub fn buffer_text(buffer: &Buffer) -> String {
    (buffer.area.y..buffer.area.bottom())
        .map(|row| row_text(buffer, row))
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_text(buffer: &Buffer, row: u16) -> String {
    let mut result = String::new();
    let mut column = buffer.area.x;
    while column < buffer.area.right() {
        let symbol = buffer[(column, row)].symbol();
        result.push_str(symbol);
        column = column.saturating_add(symbol.width().max(1) as u16);
    }
    result
}

pub fn overlay_text(app: &mut App, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    app.compositor_mut().render(area, &mut buffer);
    buffer_text(&buffer)
}

pub fn assert_visible_error(app: &mut App, target: &str, categories: &[&str]) -> TestResult {
    let rendered = overlay_text(app, 180, 36);
    let error = rendered
        .lines()
        .find(|line| line.to_lowercase().contains("error:"))
        .ok_or_else(|| format!("missing visible Error: region for {target}:\n{rendered}"))?;
    assert!(
        error.contains(target),
        "error must identify its target: {error}"
    );
    assert!(
        categories
            .iter()
            .any(|category| error.to_lowercase().contains(category)),
        "error must explain its category {categories:?}: {error}"
    );
    Ok(())
}

pub fn text(app: &mut App) -> String {
    buffer_text(&buffer(app, 120, 36))
}

pub fn assert_chooser(app: &mut App) {
    let rendered = text(app);
    assert!(
        rendered.contains("Open in agent") && rendered.contains("Show full details"),
        "actual Enter must offer the two card choices, got:\n{rendered}"
    );
}

pub fn chooser(app: &mut App) {
    assert!(press(app, KeyCode::Enter), "card Enter must be consumed");
    assert_chooser(app);
}

pub fn request_details(app: &mut App) {
    chooser(app);
    press(app, KeyCode::Down);
    press(app, KeyCode::Enter);
}

pub async fn open_details(app: &mut App) -> TestResult {
    request_details(app);
    settle(app).await
}

pub fn harvest(app: &mut App, width: u16, height: u16) -> String {
    press(app, KeyCode::Home);
    let mut result = String::new();
    for _ in 0..128 {
        result.push_str(&overlay_text(app, width, height));
        press(app, KeyCode::PageDown);
    }
    result.push_str(&overlay_text(app, width, height));
    result
}

pub fn focus_label(app: &mut App, label: &str) {
    focus_label_at(app, label, 120, 36);
}

pub fn focus_label_at(app: &mut App, label: &str, width: u16, height: u16) {
    for _ in 0..64 {
        press(app, KeyCode::Tab);
        if selected_position(&buffer(app, width, height), label).is_some() {
            return;
        }
    }
    assert!(
        selected_position(&buffer(app, width, height), label).is_some(),
        "Tab did not reveal selectable target {label}:\n{}",
        text(app)
    );
}

pub fn wheel(app: &mut App, kind: MouseEventKind) {
    event(
        app,
        Event::Mouse(MouseEvent {
            kind,
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        }),
    );
}

pub struct HeldDetails {
    pub generation: u64,
    pub actions: Vec<Action>,
    pub envelope: Option<serde_json::Value>,
}

pub async fn held_responses(
    app: &mut App,
    expected_card: &str,
    expected_error: bool,
) -> Result<HeldDetails, Box<dyn std::error::Error>> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(task) = app.next_pending_task() {
            task.await?;
        }
        let mut actions = Vec::new();
        while let Some(action) = app.try_recv_action() {
            actions.push(action);
        }
        let [Action::BoardCardDetailsLoaded {
            request_generation,
            work_unit_id,
            result,
        }] = actions.as_slice()
        else {
            return Err(
                format!("expected exactly one typed details completion, got {actions:?}").into(),
            );
        };
        assert_eq!(work_unit_id, expected_card);
        assert_eq!(
            result.is_err(),
            expected_error,
            "wrong completion result kind"
        );
        let envelope = if let Ok(payload) = result {
            let envelope: serde_json::Value = serde_json::from_str(payload)?;
            assert_eq!(envelope["id"].as_str(), Some(expected_card));
            let workspace = envelope["workspace"].as_str().ok_or("missing workspace")?;
            assert!(std::path::Path::new(workspace).is_absolute());
            assert!(envelope["workUnit"].is_object());
            assert_eq!(envelope["workUnit"]["id"].as_str(), Some(expected_card));
            Some(envelope)
        } else {
            None
        };
        Ok(HeldDetails {
            generation: *request_generation,
            actions,
            envelope,
        })
    })
    .await?
}
