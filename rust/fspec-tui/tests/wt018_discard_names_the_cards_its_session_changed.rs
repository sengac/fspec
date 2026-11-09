//! WT-018: discarding a worktree names the cards its session changed.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Harness: the real `App` dispatch over `MockBackend` (the TUI-127
//! pattern) on a paused tokio clock. The mock answers the daemon's two
//! board calls, `session_card_changes` and `move_session_cards_back`, with
//! the daemon's own functions (`codelet_fspec_core::session_cards`) over a
//! temp main checkout whose board holds what session `s1` wrote (every
//! entry it wrote carries its session id, as the WT-018 board model records
//! it), so the listing and the moves back are the real ones. Every key goes
//! through `App::handle_event`; every backend reply is awaited through the
//! App's own pending tasks and action bus.

use crate::common::TaskOutcome;
use codelet_test_helpers::{TestError, TestResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_fspec_core::session_cards;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionChangesSummary, SessionId};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::{json, Value};

mod common;
use common::MockBackend;

const MERGE_DIALOG_ID: &str = "merge-confirm-dialog";
const DISCARD_DIALOG_ID: &str = "discard-cards-dialog";
const WIDTH: u16 = 140;
const HEIGHT: u16 = 40;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

// ---------------------------------------------------------------- board

const T0: &str = "2026-11-08T00:00:00.000Z";
const T1: &str = "2026-11-08T01:00:00.000Z";
const T2: &str = "2026-11-08T02:00:00.000Z";

fn entry(state: &str, at: &str, session: Option<&str>) -> Value {
    match session {
        Some(session) => json!({"state": state, "timestamp": at, "sessionId": session}),
        None => json!({"state": state, "timestamp": at}),
    }
}

fn card(id: &str, status: &str, history: Vec<Value>) -> Value {
    json!({
        "id": id, "title": format!("Title of {id}"), "type": "story", "status": status,
        "createdAt": T0, "updatedAt": T2, "stateHistory": history
    })
}

/// STORY-001: s1 moved it from specifying to testing and implementing.
fn story_001() -> Value {
    let mut card = card(
        "STORY-001",
        "implementing",
        vec![
            entry("specifying", T0, None),
            entry("testing", T1, Some("s1")),
            entry("implementing", T2, Some("s1")),
        ],
    );
    card["editedInSessions"] = json!(["s1"]);
    card
}

/// STORY-002: s1 moved it out of backlog (no entry before the session's).
fn story_002() -> Value {
    let mut card = card(
        "STORY-002",
        "specifying",
        vec![entry("specifying", T1, Some("s1"))],
    );
    card["editedInSessions"] = json!(["s1"]);
    card
}

/// BUG-009: created by s1.
fn bug_009() -> Value {
    let mut card = card("BUG-009", "backlog", vec![entry("backlog", T1, Some("s1"))]);
    card["type"] = json!("bug");
    card["createdInSession"] = json!("s1");
    card
}

/// STORY-003: s1 only added a rule.
fn story_003() -> Value {
    let mut card = card(
        "STORY-003",
        "specifying",
        vec![entry("specifying", T0, None)],
    );
    card["rules"] = json!([{"id": 0, "text": "A rule from s1", "deleted": false, "createdAt": T1, "sessionId": "s1"}]);
    card["nextRuleId"] = json!(1);
    card["editedInSessions"] = json!(["s1"]);
    card
}

/// STORY-004: s1 moved it to testing; the user then moved it to
/// implementing from the main checkout.
fn story_004() -> Value {
    let mut card = card(
        "STORY-004",
        "implementing",
        vec![
            entry("specifying", T0, None),
            entry("testing", T1, Some("s1")),
            entry("implementing", T2, None),
        ],
    );
    card["editedInSessions"] = json!(["s1"]);
    card
}

/// STORY-005: s1 moved it to testing; moving it back to specifying fails
/// because it is blocked by STORY-006, which is not done.
fn story_005() -> Value {
    let mut card = card(
        "STORY-005",
        "testing",
        vec![
            entry("specifying", T0, None),
            entry("testing", T1, Some("s1")),
        ],
    );
    card["blockedBy"] = json!(["STORY-006"]);
    card["editedInSessions"] = json!(["s1"]);
    card
}

fn story_006() -> Value {
    card("STORY-006", "backlog", vec![entry("backlog", T0, None)])
}

/// A main checkout whose board holds `cards`.
fn main_checkout(cards: Vec<Value>) -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    let spec = main.join("spec");
    fs::create_dir_all(spec.join("features"))?;
    let mut units = serde_json::Map::new();
    let mut states: serde_json::Map<String, Value> = [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ]
    .iter()
    .map(|state| ((*state).to_string(), json!([])))
    .collect();
    for card in cards {
        let id = card["id"].as_str().unwrap_or_default().to_string();
        let status = card["status"].as_str().unwrap_or_default().to_string();
        if let Some(Value::Array(column)) = states.get_mut(&status) {
            column.push(json!(id));
        }
        units.insert(id, card);
    }
    let board = json!({
        "meta": {"version": "0.7.1", "lastUpdated": T2},
        "workUnits": units,
        "states": states
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    Ok((dir, main))
}

fn board(main: &Path) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&fs::read_to_string(
        main.join("spec").join("work-units.json"),
    )?)?)
}

fn status_of(board: &Value, id: &str) -> String {
    board["workUnits"][id]["status"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn newest_entry(board: &Value, id: &str) -> Value {
    board["workUnits"][id]["stateHistory"]
        .as_array()
        .and_then(|history| history.last())
        .cloned()
        .unwrap_or(Value::Null)
}

// ------------------------------------------------------------------ app

/// Session s1 focused on the Agent view, its worktree holding changes, the
/// mock answering the board calls from `main`'s board.
async fn session_s1_over(main: &Path) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_changes_summary(SessionChangesSummary {
        files_changed: 1,
        insertions: 3,
        deletions: 0,
        commits: vec!["abc1234".to_string()],
        files_ignored: 0,
    });
    let root = main.to_path_buf();
    mock.set_session_card_changes(move |session: &SessionId| {
        session_cards::session_card_changes(&root, &session.value).map_err(|e| e.to_string())
    });
    let root = main.to_path_buf();
    mock.set_move_session_cards_back(move |session: &SessionId| {
        session_cards::move_session_cards_back(&root, &session.value).map_err(|e| e.to_string())
    });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s1")));
    drain(&mut app).await?;
    app.dispatch(Action::OpenAgentView(Some(sid("s1"))));
    drain(&mut app).await?;
    app.navigator_mut().active_view = ViewMode::Agent;
    Ok((app, mock))
}

/// Run every pending task and every queued action until nothing moves.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..100 {
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
    Err(TestError::from("the App never settled"))
}

async fn press(app: &mut App, code: KeyCode) -> TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    drain(app).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch)).await?;
    }
    Ok(())
}

fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The screen row that names `id`.
fn row_naming(screen: &str, id: &str) -> Result<String, TestError> {
    screen
        .lines()
        .find(|line| line.contains(id))
        .map(str::to_string)
        .ok_or_else(|| TestError::from(format!("no row names {id} in:\n{screen}")))
}

fn assert_row(screen: &str, id: &str, needle: &str) -> TestResult {
    let row = row_naming(screen, id)?;
    if row.contains(needle) {
        Ok(())
    } else {
        Err(TestError::from(format!(
            "the {id} row lacks {needle:?}: {row}\n{screen}"
        )))
    }
}

fn session_text(app: &App, id: &SessionId) -> String {
    app.agent_view_store()
        .session_context_for(id)
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| {
            chunk.lines.iter().map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// /merge-worktree, then Discard (the second button) in its dialog.
async fn choose_discard(app: &mut App) -> TestResult {
    type_text(app, "/merge-worktree").await?;
    press(app, KeyCode::Enter).await?;
    if !app.compositor().contains(MERGE_DIALOG_ID) {
        return Err(TestError::from(format!(
            "/merge-worktree did not open the dialog:\n{}",
            screen(app)
        )));
    }
    press(app, KeyCode::Tab).await?;
    press(app, KeyCode::Enter).await
}

/// The discard confirmation is open; activate its button `index`
/// (0 = Discard and move back, 1 = Discard and keep statuses).
async fn pick(app: &mut App, index: usize, label: &str) -> TestResult {
    let shown = screen(app);
    if !app.compositor().contains(DISCARD_DIALOG_ID) || !shown.contains(label) {
        return Err(TestError::from(format!(
            "no discard confirmation offering {label:?}:\n{shown}"
        )));
    }
    for _ in 0..index {
        press(app, KeyCode::Tab).await?;
    }
    press(app, KeyCode::Enter).await
}

const MOVE_BACK: &str = "Discard and move back";
const KEEP: &str = "Discard and keep statuses";

// ------------------------------------------------------------ scenarios

#[tokio::test(start_paused = true)]
async fn the_discard_confirmation_lists_the_cards_the_session_moved_created_and_edited(
) -> TestResult {
    // @step Given session "s1" moved STORY-001 from specifying to implementing, moved STORY-002 from backlog to specifying, created BUG-009 and added a rule to STORY-003
    let (_dir, main) = main_checkout(vec![story_001(), story_002(), bug_009(), story_003()])?;
    let (mut app, mock) = session_s1_over(&main).await?;

    // @step When the user chooses Discard in the merge dialog of session "s1"
    choose_discard(&mut app).await?;
    let shown = screen(&mut app);
    assert!(app.compositor().contains(DISCARD_DIALOG_ID), "{shown}");
    assert_eq!(
        mock.discard_session_worktree_calls(),
        0,
        "nothing is discarded before the choice"
    );

    // @step Then the confirmation lists STORY-001 as moved, before the session: specifying
    assert_row(&shown, "STORY-001", "implementing")?;
    assert_row(&shown, "STORY-001", "moved, before the session: specifying")?;

    // @step And it lists STORY-002 as moved, before the session: backlog
    assert_row(&shown, "STORY-002", "moved, before the session: backlog")?;

    // @step And it lists BUG-009 as created and kept
    assert_row(&shown, "BUG-009", "created and kept")?;

    // @step And it lists STORY-003 as edited
    assert_row(&shown, "STORY-003", "edited")?;
    assert!(shown.contains(MOVE_BACK) && shown.contains(KEEP) && shown.contains("Cancel"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn discard_and_move_back_restores_each_cards_status_from_before_the_session() -> TestResult {
    // @step Given the discard confirmation of session "s1" lists STORY-001 (before: specifying) and STORY-002 (before: backlog)
    let (_dir, main) = main_checkout(vec![story_001(), story_002(), bug_009(), story_003()])?;
    let (mut app, mock) = session_s1_over(&main).await?;
    choose_discard(&mut app).await?;
    let shown = screen(&mut app);
    assert_row(&shown, "STORY-001", "before the session: specifying")?;
    assert_row(&shown, "STORY-002", "before the session: backlog")?;

    // @step When the user picks discard and move back
    pick(&mut app, 0, MOVE_BACK).await?;

    // @step Then the worktree is removed
    assert_eq!(mock.discard_session_worktree_calls(), 1);
    assert!(!app.compositor().contains(DISCARD_DIALOG_ID));
    assert!(session_text(&app, &sid("s1")).contains("[discard] worktree discarded"));

    // @step And STORY-001 is in specifying and STORY-002 is in backlog
    let after = board(&main)?;
    assert_eq!(status_of(&after, "STORY-001"), "specifying");
    assert_eq!(status_of(&after, "STORY-002"), "backlog");
    assert!(after["states"]["backlog"]
        .as_array()
        .is_some_and(|column| column.contains(&json!("STORY-002"))));

    // @step And each card's newest stateHistory entry records the move as a discard
    for (id, state) in [("STORY-001", "specifying"), ("STORY-002", "backlog")] {
        let newest = newest_entry(&after, id);
        assert_eq!(newest["state"], state, "{id}: {newest}");
        assert_eq!(newest["reason"], "discard", "{id}: {newest}");
        assert_eq!(newest["discardOf"], "s1", "{id}: {newest}");
    }

    // @step And BUG-009 and STORY-003's rule stay
    assert_eq!(status_of(&after, "BUG-009"), "backlog");
    assert_eq!(
        after["workUnits"]["STORY-003"]["rules"][0]["text"],
        "A rule from s1"
    );
    assert_eq!(status_of(&after, "STORY-003"), "specifying");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn discard_and_keep_statuses_changes_no_card() -> TestResult {
    // @step Given the discard confirmation of session "s1" lists moved cards
    let (_dir, main) = main_checkout(vec![story_001(), story_002()])?;
    let before = board(&main)?;
    let (mut app, mock) = session_s1_over(&main).await?;
    choose_discard(&mut app).await?;
    assert_row(&screen(&mut app), "STORY-001", "moved")?;

    // @step When the user picks discard and keep statuses
    pick(&mut app, 1, KEEP).await?;

    // @step Then the worktree is removed
    assert_eq!(mock.discard_session_worktree_calls(), 1);
    assert!(!app.compositor().contains(DISCARD_DIALOG_ID));

    // @step And every card keeps its status
    assert_eq!(mock.move_session_cards_back_calls(), 0);
    assert_eq!(board(&main)?, before);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_card_someone_else_moved_since_is_shown_and_not_moved() -> TestResult {
    // @step Given session "s1" moved STORY-004 to testing and the user then moved it to implementing from the main checkout
    let (_dir, main) = main_checkout(vec![story_004(), story_001()])?;
    let (mut app, _mock) = session_s1_over(&main).await?;

    // @step When the user chooses Discard in the merge dialog of session "s1" and picks discard and move back
    choose_discard(&mut app).await?;
    let shown = screen(&mut app);
    pick(&mut app, 0, MOVE_BACK).await?;

    // @step Then the confirmation shows STORY-004 as changed by someone else since
    assert_row(&shown, "STORY-004", "changed by someone else since")?;

    // @step And STORY-004 stays in implementing
    let after = board(&main)?;
    assert_eq!(status_of(&after, "STORY-004"), "implementing");
    assert_eq!(
        newest_entry(&after, "STORY-004"),
        entry("implementing", T2, None)
    );
    assert_eq!(status_of(&after, "STORY-001"), "specifying");
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_move_back_that_fails_is_reported_and_the_other_cards_still_move() -> TestResult {
    // @step Given the discard confirmation of session "s1" lists STORY-001 and STORY-005
    let (_dir, main) = main_checkout(vec![story_001(), story_005(), story_006()])?;
    let (mut app, _mock) = session_s1_over(&main).await?;
    choose_discard(&mut app).await?;
    let shown = screen(&mut app);
    assert_row(&shown, "STORY-001", "moved")?;
    assert_row(&shown, "STORY-005", "moved, before the session: specifying")?;

    // @step And moving STORY-005 back fails
    // (STORY-005 is blocked by STORY-006, which is not done, so the
    // dependency gate refuses specifying.)

    // @step When the user picks discard and move back
    pick(&mut app, 0, MOVE_BACK).await?;

    // @step Then the notice names STORY-005 and the error
    let notices = session_text(&app, &sid("s1"));
    let line = notices
        .lines()
        .find(|line| line.contains("STORY-005"))
        .ok_or_else(|| TestError::from(format!("no notice names STORY-005:\n{notices}")))?;
    assert!(line.contains("not moved back"), "{line}");
    assert!(
        line.contains("blocked by incomplete dependencies"),
        "{line}"
    );
    assert_eq!(status_of(&board(&main)?, "STORY-005"), "testing");

    // @step And STORY-001 is moved back
    assert_eq!(status_of(&board(&main)?, "STORY-001"), "specifying");
    assert!(
        notices.contains("STORY-001 moved back to specifying"),
        "{notices}"
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_session_that_changed_no_card_discards_as_before() -> TestResult {
    // @step Given session "s1" moved, created and edited no card
    let (_dir, main) = main_checkout(vec![story_006()])?;
    let (mut app, mock) = session_s1_over(&main).await?;

    // @step When the user chooses Discard in the merge dialog of session "s1"
    choose_discard(&mut app).await?;

    // @step Then no card list is shown
    assert!(
        !app.compositor().contains(DISCARD_DIALOG_ID),
        "{}",
        screen(&mut app)
    );
    assert_eq!(mock.session_card_changes_calls(), 1);

    // @step And the worktree is removed as before
    assert_eq!(mock.discard_session_worktree_calls(), 1);
    assert_eq!(mock.move_session_cards_back_calls(), 0);
    assert!(session_text(&app, &sid("s1")).contains("[discard] worktree discarded"));
    Ok(())
}
