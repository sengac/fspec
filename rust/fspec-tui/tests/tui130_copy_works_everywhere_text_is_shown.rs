//! TUI-130 — copy works everywhere text is shown.
//!
//! Feature: spec/features/copy-works-everywhere-text-is-shown.feature
//!
//! Every test drives the real `App` over a `MockBackend` from the key, paste
//! or mouse event to the OSC 52 bytes the App writes to its one clipboard
//! writer (`App::set_clipboard_writer_for_test`), and reads the confirmation
//! from the rows `App::render` paints. Time is tokio's paused clock.

use std::sync::Arc;

use codelet_fspec_tui::components::notification_dialog::NotificationDialog;
use codelet_fspec_tui::components::session_storage_dialog::SESSION_STORAGE_DIALOG_ID;
use codelet_fspec_tui::views::agent::session_worktrees_dialog::SESSION_WORKTREES_DIALOG_ID;
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::model_selector::CustomModelMode;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    BlocklistRuleInfo, ChangedFile, CheckpointInfo, MergeOutcome, MergeStatus, ModelEntry,
    SessionChangesSummary, SessionId, SessionStatus, SessionWorktreeInfo, StreamChunk,
    WorkUnitInfo, WorkspaceEntry,
};
use codelet_sessions::profile_sections::build_profile_provider_info;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::clipboard::{osc52, Clipboard};
use common::MockBackend;
use common::TaskOutcome;

fn sid(id: &str) -> SessionId {
    SessionId::new(id)
}

struct Fixture {
    app: App,
    mock: Arc<MockBackend>,
    clipboard: Clipboard,
}

fn fixture_with(mock: Arc<MockBackend>) -> Fixture {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let clipboard = Clipboard::attach(&mut app);
    Fixture {
        app,
        mock,
        clipboard,
    }
}

fn fixture() -> Fixture {
    fixture_with(Arc::new(MockBackend::new()))
}

/// An App on the agent view of session s-1.
fn agent_fixture() -> Fixture {
    let mut fx = fixture();
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    fx.app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    fx
}

/// Run every parked task and queued action until the App is quiet.
async fn settle(app: &mut App) -> TestResult {
    for _ in 0..64 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Ok(())
}

async fn key(fx: &mut Fixture, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    let _ = fx
        .app
        .handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    settle(&mut fx.app).await
}

async fn press(fx: &mut Fixture, ch: char) -> TestResult {
    let modifiers = if ch.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    key(fx, KeyCode::Char(ch), modifiers).await
}

async fn ctrl_c(fx: &mut Fixture) -> TestResult {
    key(fx, KeyCode::Char('c'), KeyModifiers::CONTROL).await
}

async fn mouse(fx: &mut Fixture, kind: MouseEventKind, column: u16, row: u16) -> TestResult {
    let _ = fx.app.handle_event(&Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }));
    settle(&mut fx.app).await
}

/// The last thing the App put on the clipboard is exactly `text`.
fn copied(fx: &Fixture, text: &str) -> Result<bool, TestError> {
    Ok(fx.clipboard.bytes()?.ends_with(&osc52(text)))
}

fn rows(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

fn screen_text(app: &mut App, width: u16, height: u16) -> String {
    rows(app, width, height).join("\n")
}

/// The (column, row) where `needle` starts on screen.
fn find_on_screen(rows: &[String], needle: &str) -> Option<(u16, u16)> {
    rows.iter().enumerate().find_map(|(y, row)| {
        row.find(needle)
            .map(|byte| (row[..byte].chars().count() as u16, y as u16))
    })
}

/// The cell just past the text of the last row of the run of non-blank
/// rows starting at `row` (a drag selects up to, not including, its end).
fn end_of_text(rows: &[String], row: u16) -> (u16, u16) {
    let mut y = row as usize;
    while y + 1 < rows.len() && !rows[y + 1].trim().is_empty() {
        y += 1;
    }
    (rows[y].trim_end().chars().count() as u16, y as u16)
}

/// Drag the left button from `from` to `to` (the end cell is not selected).
async fn drag(fx: &mut Fixture, from: (u16, u16), to: (u16, u16)) -> TestResult {
    mouse(fx, MouseEventKind::Down(MouseButton::Left), from.0, from.1).await?;
    mouse(fx, MouseEventKind::Drag(MouseButton::Left), to.0, to.1).await?;
    mouse(fx, MouseEventKind::Up(MouseButton::Left), to.0, to.1).await
}

/// Words of `n` characters in total (for wrapped prose).
fn prose(n: usize) -> String {
    let mut out = String::new();
    let mut i = 0;
    while out.len() < n {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&format!("word{i}"));
        i += 1;
    }
    out.truncate(n);
    out.trim_end().to_string()
}

fn long_token(prefix: &str, n: usize) -> String {
    let mut out = prefix.to_string();
    let mut i = 0u32;
    while out.len() < n {
        out.push(char::from(b'a' + (i % 26) as u8));
        i += 1;
    }
    out
}

/// Push an assistant reply and finish the turn on session s-1.
fn reply(fx: &mut Fixture, text: &str) {
    fx.app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text(text.to_string()),
    ));
    fx.app
        .dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::Done));
}

/// Decode the last OSC 52 sequence in `raw`.
fn decode_last_copy(raw: &str) -> Result<String, TestError> {
    use base64::Engine as _;
    let start = raw.rfind("\x1b]52;c;").or_fail("an OSC 52 copy")? + "\x1b]52;c;".len();
    let end = raw[start..].find('\x07').or_fail("the OSC 52 terminator")? + start;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&raw[start..end])
        .or_fail("base64")?;
    Ok(String::from_utf8(bytes).or_fail("utf8 copy")?)
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken pipe"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("broken pipe"))
    }
}

#[tokio::test(start_paused = true)]
async fn every_copy_is_confirmed_with_what_the_app_knows() -> TestResult {
    // @step Given the App's clipboard writer accepts writes
    let mut fx = fixture();

    // @step When "abc" is copied
    fx.app.dispatch(Action::CopyToClipboard("abc".into()));

    // @step Then the clipboard receives exactly "abc"
    assert_eq!(fx.clipboard.bytes()?, osc52("abc"));
    // @step And the screen shows "Sent 3 characters to the terminal clipboard (OSC 52)"
    let sent = "Sent 3 characters to the terminal clipboard (OSC 52)";
    assert!(screen_text(&mut fx.app, 120, 30).contains(sent));
    // @step And the confirmation is gone 2 seconds later
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    assert!(!screen_text(&mut fx.app, 120, 30).contains(sent));

    // @step When the clipboard writer fails with "broken pipe" and "abc" is copied
    fx.app
        .set_clipboard_writer_for_test(Box::new(FailingWriter));
    fx.app.dispatch(Action::CopyToClipboard("abc".into()));

    // @step Then the screen shows "Copy failed: broken pipe"
    assert!(screen_text(&mut fx.app, 120, 30).contains("Copy failed: broken pipe"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_api_error_dialog_copies_its_whole_message() -> TestResult {
    // @step Given the API Error dialog shows a 300-character error
    let mut fx = agent_fixture();
    let error = prose(300);
    fx.app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::Error {
            error: error.clone(),
        },
    ));
    settle(&mut fx.app).await?;
    let expected = format!("API Error: {error}");

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives "API Error: " followed by the whole error
    assert_eq!(fx.clipboard.bytes()?, osc52(&expected));

    // @step When I press Ctrl+C
    fx.clipboard.clear()?;
    ctrl_c(&mut fx).await?;

    // @step Then the clipboard receives the same text again
    assert_eq!(fx.clipboard.bytes()?, osc52(&expected));
    // @step And the dialog footer lists "c copy"
    assert!(screen_text(&mut fx.app, 120, 40).contains("c copy"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_session_error_dialog_copies_rows_scrolled_out_of_view() -> TestResult {
    // @step Given the Session error dialog shows a message longer than the dialog
    let mut fx = fixture();
    let message = (0..40)
        .map(|i| format!("line {i}: {}", prose(90)))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nPath: /very/long/path/to/the/session/store/manifest.json";
    fx.app
        .dispatch(Action::SessionOperationFailed(message.clone()));
    settle(&mut fx.app).await?;
    assert!(fx.app.compositor().contains(SESSION_STORAGE_DIALOG_ID));
    let _ = rows(&mut fx.app, 80, 24);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the whole message
    assert_eq!(fx.clipboard.bytes()?, osc52(&message));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_notification_dialog_copies_its_whole_sentence() -> TestResult {
    // @step Given the daemon-saved Notification dialog is shown
    let mut fx = fixture();
    let sentence = "Saved daemon work. The TUI stays connected to the daemon it is using now; \
                    the new address and token are used the next time it connects to work.";
    fx.app.compositor_mut().push(Box::new(
        NotificationDialog::info(sentence).with_auto_dismiss_ms(0),
    ));

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives its whole sentence
    assert_eq!(fx.clipboard.bytes()?, osc52(sentence));
    Ok(())
}

fn unit(id: &str, title: &str, description: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(),
        title: title.into(),
        status: "backlog".into(),
        work_type: "story".into(),
        description: Some(description.into()),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// The board with one card selected in backlog, in workspace /work/alpha.
fn board_fixture(card: WorkUnitInfo) -> Fixture {
    let mut fx = fixture();
    fx.app.dispatch(Action::WorkUnitsLoaded(vec![card]));
    fx.app
        .board_store_mut()
        .set_workspaces(vec![WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        }]);
    fx.app.board_store_mut().set_focused_column("backlog");
    fx.app
        .board_store_mut()
        .set_selected_index_for("backlog", 0);
    fx
}

#[tokio::test(start_paused = true)]
async fn board_card_details_copy_the_whole_page_and_the_card_id() -> TestResult {
    // @step Given the board card details are open for a card with a 500-character description
    let description = prose(500);
    let mut fx = board_fixture(unit("AUTH-001", "Sign in", "short"));
    let _ = rows(&mut fx.app, 120, 36);
    key(&mut fx, KeyCode::Enter, KeyModifiers::NONE).await?;
    let _ = fx.app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Down,
        KeyModifiers::NONE,
    )));
    let _ = fx.app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    // Serve the details document with the long description.
    let mut generation = None;
    for _ in 0..16 {
        while let Some(handle) = fx.app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = fx.app.try_recv_action() {
            match action {
                Action::BoardCardDetailsLoaded {
                    request_generation, ..
                } => generation = Some(request_generation),
                other => fx.app.dispatch(other),
            }
        }
        if generation.is_some() {
            break;
        }
    }
    let request_generation = generation.or_fail("the card details were requested")?;
    let document = serde_json::json!({
        "id": "AUTH-001",
        "workspace": "/work/alpha",
        "workUnit": {"id": "AUTH-001", "title": "Sign in", "type": "story",
                     "status": "backlog", "description": description},
    });
    fx.app.dispatch(Action::BoardCardDetailsLoaded {
        request_generation,
        work_unit_id: "AUTH-001".into(),
        result: Ok(document.to_string()),
    });
    settle(&mut fx.app).await?;
    let _ = rows(&mut fx.app, 80, 24);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the card id, its title and the whole description
    let text = String::from_utf8(fx.clipboard.bytes()?).or_fail("utf8")?;
    let decoded = decode_last_copy(&text)?;
    assert!(decoded.contains("AUTH-001"), "{decoded}");
    assert!(decoded.contains("Sign in"), "{decoded}");
    let flat: String = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains(&description), "{decoded}");

    // @step When I press y
    press(&mut fx, 'y').await?;

    // @step Then the clipboard receives the card id
    assert!(copied(&fx, "AUTH-001")?);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_long_press_on_the_board_strip_copies_the_whole_field_through_the_app() -> TestResult {
    // @step Given the board details strip shows a card with a 500-character description
    let description = prose(500);
    let mut fx = board_fixture(unit("AUTH-001", "Sign in", &description));
    let screen = rows(&mut fx.app, 120, 36);
    let (_, title_row) = find_on_screen(&screen, "AUTH-001: Sign in").or_fail("strip title row")?;
    let description_row = title_row + 1;

    // @step When I long-press the strip's description row
    mouse(
        &mut fx,
        MouseEventKind::Down(MouseButton::Left),
        5,
        description_row,
    )
    .await?;
    fx.app
        .poll_selection_tick_after_for_test(std::time::Duration::from_millis(450));
    settle(&mut fx.app).await?;
    mouse(
        &mut fx,
        MouseEventKind::Up(MouseButton::Left),
        5,
        description_row,
    )
    .await?;

    // @step Then the App clipboard receives the whole description
    assert_eq!(fx.clipboard.bytes()?, osc52(&description));
    // @step And the screen shows "Sent 500 characters to the terminal clipboard (OSC 52)"
    let count = description.chars().count();
    assert!(screen_text(&mut fx.app, 120, 36).contains(&format!(
        "Sent {count} characters to the terminal clipboard (OSC 52)"
    )));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_wrapped_url_copied_from_the_scrollback_has_no_line_breaks() -> TestResult {
    // @step Given the scrollback shows a 200-character URL that wraps over 3 rows at 80 columns
    let mut fx = agent_fixture();
    let url = long_token("https://example.com/download?token=", 200);
    reply(&mut fx, &url);
    let screen = rows(&mut fx.app, 80, 24);
    let start = find_on_screen(&screen, "https://example.com/").or_fail("URL on screen")?;
    let end = end_of_text(&screen, start.1);
    assert!(
        end.1 >= start.1 + 2,
        "the URL wraps over 3 rows:\n{}",
        screen.join("\n")
    );

    // @step When I drag over all its rows
    drag(&mut fx, start, end).await?;

    // @step Then the clipboard receives the URL with no line break
    assert!(copied(&fx, &url)?, "{:?}", fx.clipboard.bytes()?);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn wrapped_words_are_joined_by_a_space_and_real_line_breaks_are_kept() -> TestResult {
    // @step Given the scrollback shows an assistant reply whose second line wraps
    let mut fx = agent_fixture();
    let second = prose(170);
    let text = format!("first line\n{second}");
    reply(&mut fx, &text);
    let screen = rows(&mut fx.app, 80, 24);
    let start = find_on_screen(&screen, "first line").or_fail("first line on screen")?;
    let end = end_of_text(&screen, start.1);
    assert!(end.1 >= start.1 + 3, "the second line wraps");

    // @step When I drag over all its rows
    drag(&mut fx, start, end).await?;

    // @step Then the clipboard holds exactly one line break, where the reply had one
    let copy = decode_last_copy(&String::from_utf8(fx.clipboard.bytes()?).or_fail("utf8")?)?;
    assert_eq!(copy.matches('\n').count(), 1, "{copy:?}");
    // @step And the wrapped words are joined by one space
    assert_eq!(copy, text);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_turn_modal_copies_the_whole_turn() -> TestResult {
    // @step Given the turn modal is open on a long assistant reply
    let mut fx = agent_fixture();
    let text = (0..60)
        .map(|i| format!("row {i} {}", prose(100)))
        .collect::<Vec<_>>()
        .join("\n");
    reply(&mut fx, &text);
    let _ = rows(&mut fx.app, 80, 24);
    key(&mut fx, KeyCode::Tab, KeyModifiers::NONE).await?;
    key(&mut fx, KeyCode::Enter, KeyModifiers::NONE).await?;
    assert!(fx.app.navigator().agent.turn_modal_seq.is_some());
    let _ = rows(&mut fx.app, 80, 24);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the whole turn text
    assert_eq!(fx.clipboard.bytes()?, osc52(&text));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn ctrl_c_copies_an_agent_view_selection_instead_of_interrupting() -> TestResult {
    // @step Given the agent view has a scrollback selection
    let mut fx = agent_fixture();
    reply(&mut fx, "select me please");
    settle(&mut fx.app).await?;
    fx.app
        .agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    let screen = rows(&mut fx.app, 80, 24);
    let start = find_on_screen(&screen, "select me").or_fail("reply on screen")?;
    drag(&mut fx, start, (start.0 + 9, start.1)).await?;
    fx.clipboard.clear()?;

    // @step When I press Ctrl+C
    ctrl_c(&mut fx).await?;

    // @step Then the clipboard receives the selection
    assert_eq!(fx.clipboard.bytes()?, osc52("select me"));
    // @step And no interrupt is sent
    assert_eq!(fx.mock.interrupt_calls(), 0);

    // @step When I press Ctrl+C again
    ctrl_c(&mut fx).await?;

    // @step Then an interrupt is sent
    // The App sends the interrupt from a detached task.
    let mock = fx.mock.clone();
    codelet_test_helpers::deadlock_guard("the interrupt", async move {
        while mock.interrupt_calls() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(fx.mock.interrupt_calls(), 1);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn changed_files_copy_the_full_path_and_the_diff() -> TestResult {
    // @step Given the changed files view lists a file with a 120-character path
    let path = long_token("rust/fspec-tui/src/views/", 117) + ".rs";
    let mock = Arc::new(MockBackend::new());
    mock.set_changed_files(vec![ChangedFile {
        path: path.clone(),
        change_type: "M".into(),
        staged: false,
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenChangedFilesView);
    settle(&mut fx.app).await?;
    let diff = "@@ -1 +1 @@\n-old line\n+new line".to_string();
    fx.app.dispatch(Action::FileDiffLoaded {
        path: path.clone(),
        diff: Some(diff.clone()),
    });
    settle(&mut fx.app).await?;
    assert_eq!(fx.app.navigator().active_view, ViewMode::ChangedFiles);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the full path
    assert!(copied(&fx, &path)?);

    // @step When I press C
    press(&mut fx, 'C').await?;

    // @step Then the clipboard receives the shown diff lines
    assert!(copied(&fx, &diff)?, "{:?}", fx.clipboard.bytes()?);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn checkpoints_copy_the_selected_checkpoint_name() -> TestResult {
    // @step Given the checkpoints view lists a checkpoint
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoints(vec![CheckpointInfo {
        work_unit_id: "TUI-130".into(),
        name: "before-copy-refactor".into(),
        timestamp: "2026-11-07T00:00:00.000Z".into(),
        is_automatic: false,
    }]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenCheckpointsView);
    settle(&mut fx.app).await?;
    assert_eq!(fx.app.navigator().active_view, ViewMode::Checkpoints);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the checkpoint's name
    assert_eq!(fx.clipboard.bytes()?, osc52("before-copy-refactor"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_worktrees_dialog_copies_the_chosen_worktree_path() -> TestResult {
    // @step Given the /worktrees dialog lists two worktrees with 90-character paths
    let mock = Arc::new(MockBackend::new());
    let path = |n: &str| {
        long_token(
            &format!("/home/user/projects/repo/.fspec/worktrees/{n}-"),
            90,
        )
    };
    let row = |n: &str| SessionWorktreeInfo {
        session_id: sid(n),
        worktree_path: path(n),
        base_commit: "abc1234".into(),
        head_commit: "def5678".into(),
        dirty: false,
    };
    mock.seed_session_worktrees(vec![row("w-1"), row("w-2")]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    settle(&mut fx.app).await?;
    let session = fx.app.current_session();
    fx.app.dispatch(Action::SlashCommandSelected(
        session,
        SlashCommandAction::Worktrees,
    ));
    settle(&mut fx.app).await?;
    assert!(fx.app.compositor().contains(SESSION_WORKTREES_DIALOG_ID));

    // @step When I press Down and then c
    key(&mut fx, KeyCode::Down, KeyModifiers::NONE).await?;
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the second worktree's path
    assert_eq!(fx.clipboard.bytes()?, osc52(&path("w-2")));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_model_selector_copies_the_focused_model_and_takes_a_paste_in_its_form() -> TestResult {
    // @step Given the model selector is focused on a model of a provider
    let mock = Arc::new(MockBackend::new());
    let models = vec![ModelEntry {
        id: "gpt-5-codex".into(),
        display_name: "GPT-5 Codex".into(),
        context_window: Some(400_000),
        ..ModelEntry::default()
    }];
    let provider = build_profile_provider_info("openai", "my-profile", models, false);
    let provider_key = provider.key.clone();
    mock.seed_providers(vec![provider]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::OpenModelSelectorView);
    settle(&mut fx.app).await?;
    assert_eq!(fx.app.navigator().active_view, ViewMode::ModelSelector);
    key(&mut fx, KeyCode::Right, KeyModifiers::NONE).await?;
    key(&mut fx, KeyCode::Down, KeyModifiers::NONE).await?;

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives "provider/model-id" for that model
    assert_eq!(
        fx.clipboard.bytes()?,
        osc52(&format!("{provider_key}/gpt-5-codex"))
    );

    // @step When I open the add custom model form and paste "gpt-x"
    press(&mut fx, 'a').await?;
    assert!(matches!(
        fx.app.navigator().model_selector.custom_model_mode(),
        CustomModelMode::Add { .. }
    ));
    let _ = fx.app.handle_paste("gpt-x");
    settle(&mut fx.app).await?;

    // @step Then the focused form field holds "gpt-x"
    assert_eq!(fx.app.navigator().model_selector.form().id, "gpt-x");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_blocklist_copies_the_rule_pattern_and_the_whole_detail() -> TestResult {
    // @step Given the blocklist is focused on a rule
    let mock = Arc::new(MockBackend::new());
    let rule = BlocklistRuleInfo {
        id: "cat-block".into(),
        pattern: "^cat\\s+".into(),
        action: "block".into(),
        reason: "reason for cat-block".into(),
        guidance: Some("guidance for cat-block".into()),
        source: "project".into(),
    };
    mock.seed_blocklist_rules(vec![rule]);
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    settle(&mut fx.app).await?;
    let session = fx.app.current_session();
    fx.app.dispatch(Action::SlashCommandSelected(
        session,
        SlashCommandAction::Blocklist,
    ));
    settle(&mut fx.app).await?;
    assert_eq!(fx.app.navigator().active_view, ViewMode::Blocklist);

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the rule's pattern
    assert_eq!(fx.clipboard.bytes()?, osc52("^cat\\s+"));

    // @step When I press C
    press(&mut fx, 'C').await?;

    // @step Then the clipboard receives the rule's ID, pattern, reason and guidance
    let detail = decode_last_copy(&String::from_utf8(fx.clipboard.bytes()?).or_fail("utf8")?)?;
    for part in [
        "cat-block",
        "^cat\\s+",
        "reason for cat-block",
        "guidance for cat-block",
    ] {
        assert!(detail.contains(part), "{detail}");
    }
    Ok(())
}

#[tokio::test]
async fn a_viewer_that_cannot_open_names_its_url() -> TestResult {
    // @step Given the browser launcher fails with "no display"
    let mock = Arc::new(MockBackend::new());
    mock.script_card_attachment(Ok(b"# Foundation\n".to_vec()));
    let mut fx = fixture_with(mock);
    fx.app
        .set_board_attachment_launcher_for_test(Arc::new(|_url: &str| Err("no display".into())));
    fx.app
        .board_store_mut()
        .set_workspaces(vec![WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        }]);
    fx.app.dispatch(Action::OpenWorkspaceDropdown);
    fx.app.dispatch(Action::ChooseWorkspace(Some(0)));
    settle(&mut fx.app).await?;

    // @step When I open the FOUNDATION viewer
    fx.app.dispatch(Action::OpenFoundation);
    codelet_test_helpers::deadlock_guard("the viewer error", async {
        while fx.app.session_operation_errors().is_empty() {
            settle(&mut fx.app).await?;
            tokio::task::yield_now().await;
        }
        Ok::<(), TestError>(())
    })
    .await??;

    // @step Then the error shown names the "http://127.0.0.1:" viewer URL
    let errors = fx.app.session_operation_errors().join("\n");
    assert!(errors.contains("no display"), "{errors}");
    assert!(errors.contains("http://127.0.0.1:"), "{errors}");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_merge_commit_sha_stays_on_the_board_and_can_be_copied() -> TestResult {
    // @step Given a worktree merge returns commit "abc123"
    let mock = Arc::new(MockBackend::new());
    mock.seed_merge_outcome(MergeOutcome {
        status: MergeStatus::Success,
        conflicts: Vec::new(),
        merge_commit: Some("abc123".into()),
        worktree_path: Some("/repo/.fspec/worktrees/s-1".into()),
    });
    let mut fx = fixture_with(mock);
    fx.app.dispatch(Action::SessionCreated(sid("s-1")));
    fx.app.navigator_mut().active_view = ViewMode::Agent;
    settle(&mut fx.app).await?;
    fx.app.dispatch(Action::OpenMergeConfirmDialog {
        session_id: sid("s-1"),
        summary: SessionChangesSummary {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
            commits: vec!["abc123".into()],
            files_ignored: 0,
        },
    });
    settle(&mut fx.app).await?;

    // @step When the session returns to the board
    fx.app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    settle(&mut fx.app).await?;
    assert_eq!(fx.app.navigator().active_view, ViewMode::Board);

    // @step Then a Notification dialog on the board shows "abc123"
    let screen = screen_text(&mut fx.app, 120, 36);
    assert!(screen.contains("abc123"), "{screen}");

    // @step When I press c
    press(&mut fx, 'c').await?;

    // @step Then the clipboard receives the notification text
    let copy = decode_last_copy(&String::from_utf8(fx.clipboard.bytes()?).or_fail("utf8")?)?;
    assert!(copy.contains("abc123"), "{copy}");
    Ok(())
}
