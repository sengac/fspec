//! TUI-123 — the daemon form (masked token, pasted newlines, name clashes,
//! refusals on their field, connect after save, rename, editing the
//! connected daemon), removal with confirmation and narrow terminals.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

use codelet_test_helpers::{OrFail, TestResult};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use codelet_common::saved_daemons;
use codelet_fspec_tui::daemons::{DaemonForm, TokenChange};
use codelet_fspec_tui::Action;
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode};

mod common;
mod tui123_support;
use common::MockBackend;
use tui123_support::*;

const FORM: &str = "daemon-form-dialog";
const CONFIRM: &str = "daemon-remove-confirm-dialog";
const TOKEN: &str = "secret-token";
const DOTS: &str = "••••••••••••";

#[tokio::test(flavor = "current_thread")]
async fn the_token_is_masked_everywhere() -> TestResult {
    // @step Given the add daemon dialog is open
    let work = Arc::new(MockBackend::new());
    let mut fx = fixture(Setup {
        outcomes: HashMap::from([("work".to_string(), Outcome::Backend(work))]),
        ..Setup::default()
    })
    .await?;

    // @step When the user enters name "work", URL "wss://fspec.example.com" and token "secret-token"
    fill_add_form(&mut fx.app, "work", "wss://fspec.example.com", TOKEN).await?;
    let typed = text(&render(&mut fx.app, 120, 30));
    press(&mut fx.app, KeyCode::Enter).await?;
    let saved_frame = text(&render(&mut fx.app, 120, 30));

    // @step Then the token row shows 12 dots
    let token_row = typed
        .lines()
        .find(|line| line.contains("Token"))
        .or_fail("token row")?;
    assert!(token_row.contains(DOTS), "{token_row}");
    assert!(!token_row.contains(&format!("{DOTS}•")), "{token_row}");

    // @step And "secret-token" appears in no frame and in no Debug output of the saved form
    assert!(!typed.contains(TOKEN), "{typed}");
    assert!(!saved_frame.contains(TOKEN), "{saved_frame}");
    let action = Action::SaveDaemon {
        previous: None,
        form: DaemonForm {
            name: "work".into(),
            url: "wss://fspec.example.com".into(),
            token: TokenChange::Set(TOKEN.into()),
        },
    };
    let debug = format!("{action:?}");
    assert!(!debug.contains(TOKEN), "{debug}");
    let saved = saved_daemons::find(&fx.path, "work")?.or_fail("work saved")?;
    assert!(!format!("{saved:?}").contains(TOKEN));
    assert_eq!(saved.token.as_deref(), Some(TOKEN));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pasted_newlines_are_removed() -> TestResult {
    // @step Given the add daemon dialog is open on the URL field
    let mut fx = fixture(Setup::default()).await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Char('a')).await?;
    type_text(&mut fx.app, "work").await?;
    press(&mut fx.app, KeyCode::Tab).await?;

    // @step When the user pastes "wss://fspec.example.com" followed by a newline
    fx.app
        .handle_event(&Event::Paste("wss://fspec.example.com\n".into()));
    settle(&mut fx.app).await?;

    // @step Then the URL field holds "wss://fspec.example.com"
    press(&mut fx.app, KeyCode::Tab).await?;
    type_text(&mut fx.app, TOKEN).await?;
    press(&mut fx.app, KeyCode::Enter).await?;
    let saved = saved_daemons::find(&fx.path, "work")?.or_fail("work saved")?;
    assert_eq!(saved.url, "wss://fspec.example.com");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn daemon_names_are_unique_against_saved_and_local_daemons_ignoring_case() -> TestResult {
    // @step Given saved daemon "work" and local daemon "alpha"
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some("t"))],
        ..Setup::default()
    })
    .await?;

    // @step When the user adds "WORK" and then "Alpha"
    fill_add_form(&mut fx.app, "WORK", "ws://127.0.0.1:4100", "").await?;
    press(&mut fx.app, KeyCode::Enter).await?;
    let first = text(&render(&mut fx.app, 160, 30));
    let first_open = fx.app.compositor().contains(FORM);
    press(&mut fx.app, KeyCode::Esc).await?;
    fill_add_form(&mut fx.app, "Alpha", "ws://127.0.0.1:4101", "").await?;
    press(&mut fx.app, KeyCode::Enter).await?;
    let second = text(&render(&mut fx.app, 160, 30));
    let second_open = fx.app.compositor().contains(FORM);

    // @step Then "WORK" is refused naming the saved daemon "work"
    assert!(first_open);
    assert!(
        first.contains("a daemon named work is already saved"),
        "{first}"
    );

    // @step And "Alpha" is refused naming the local daemon "alpha"
    assert!(second_open);
    assert!(
        second.contains("alpha is the name of a local daemon"),
        "{second}"
    );

    // @step And nothing new is saved
    let names: Vec<String> = saved_daemons::list(&fx.path)?
        .into_iter()
        .map(|daemon| daemon.name)
        .collect();
    assert_eq!(names, vec!["work".to_string()]);
    assert_eq!(fx.connects.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_refused_save_reopens_the_dialog_with_everything_typed() -> TestResult {
    // @step Given the add daemon dialog is open
    let mut fx = fixture(Setup::default()).await?;

    // @step When the user saves "lan" at "ws://10.0.0.5:9000" with token "secret-token"
    fill_add_form(&mut fx.app, "lan", "ws://10.0.0.5:9000", TOKEN).await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then the dialog reopens with name "lan", URL "ws://10.0.0.5:9000" and 12 token dots
    assert!(fx.app.compositor().contains(FORM));
    let buf = render(&mut fx.app, 120, 30);
    let screen = text(&buf);
    assert!(screen.contains("Name   lan"), "{screen}");
    assert!(screen.contains(DOTS), "{screen}");
    assert!(!screen.contains(TOKEN), "{screen}");

    // @step And the reason is shown under the URL field
    let url_row = find_row(&buf, "URL    ws://10.0.0.5:9000").or_fail("URL row")?;
    let reason = row(&buf, url_row + 1);
    assert!(reason.contains("wss://"), "{screen}");
    let token_row = find_row(&buf, "Token").or_fail("token row")?;
    assert!(token_row > url_row + 1, "{screen}");

    // @step And daemons.json is unchanged
    assert!(!fx.path.exists());
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn saving_a_new_daemon_connects_to_it() -> TestResult {
    // @step Given the board is connected to "alpha" and daemon "work" is reachable
    let work = Arc::new(MockBackend::new());
    let mut fx = fixture(Setup {
        outcomes: HashMap::from([("work".to_string(), Outcome::Backend(work))]),
        ..Setup::default()
    })
    .await?;

    // @step When the user adds daemon "work"
    fill_add_form(&mut fx.app, "work", "ws://127.0.0.1:4100", "").await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then "work" is saved
    assert!(saved_daemons::find(&fx.path, "work")?.is_some());

    // @step And the header reads "Daemon: work ▾"
    assert_eq!(fx.app.board_store().connected_daemon(), Some("work"));
    let screen = text(&render(&mut fx.app, 120, 24));
    assert!(screen.contains("Daemon: work ▾"), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_new_daemon_that_cannot_be_reached_stays_saved() -> TestResult {
    // @step Given the board is connected to "alpha" and daemon "offline" refuses connections
    let mut fx = fixture(Setup {
        outcomes: HashMap::from([("offline".to_string(), Outcome::Refused)]),
        ..Setup::default()
    })
    .await?;

    // @step When the user adds daemon "offline"
    fill_add_form(&mut fx.app, "offline", "ws://127.0.0.1:4101", "").await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then "offline" is saved
    assert!(saved_daemons::find(&fx.path, "offline")?.is_some());

    // @step And the header still reads "Daemon: alpha ▾"
    assert_eq!(fx.app.board_store().connected_daemon(), Some("alpha"));

    // @step And an error says "Saved daemon offline, but could not connect" and "connection refused"
    let screen = text(&render(&mut fx.app, 240, 40));
    assert!(
        screen.contains("Saved daemon offline, but could not connect"),
        "{screen}"
    );
    assert!(screen.contains("connection refused"), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn renaming_a_saved_daemon_replaces_it_and_keeps_its_token() -> TestResult {
    // @step Given saved daemon "work" at "wss://fspec.example.com" with token "secret-token"
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some(TOKEN))],
        ..Setup::default()
    })
    .await?;

    // @step When the user edits "work" and renames it to "work2" leaving the token empty
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Down).await?;
    press(&mut fx.app, KeyCode::Char('e')).await?;
    let edit = text(&render(&mut fx.app, 120, 30));
    backspace(&mut fx.app, 4).await?;
    type_text(&mut fx.app, "work2").await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then the edit dialog showed URL "wss://fspec.example.com" and an empty token row
    assert!(edit.contains("URL    wss://fspec.example.com"), "{edit}");
    assert!(edit.contains("(empty keeps the saved token)"), "{edit}");
    assert!(!edit.contains(TOKEN), "{edit}");

    // @step And only "work2" is saved, with token "secret-token"
    assert_eq!(
        saved_daemons::list(&fx.path)?,
        vec![saved("work2", "wss://fspec.example.com", Some(TOKEN))]
    );
    assert_eq!(fx.connects.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn editing_the_connected_daemon_applies_on_the_next_connect() -> TestResult {
    // @step Given the board is connected to saved daemon "work"
    let url = "wss://fspec.example.com";
    let mut fx = fixture(Setup {
        connected: "work".into(),
        saved: vec![saved("work", url, Some(TOKEN))],
        ..Setup::default()
    })
    .await?;

    // @step When the user edits "work" to "wss://fspec2.example.com" and renames it to "work2"
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Char('e')).await?;
    backspace(&mut fx.app, 4).await?;
    type_text(&mut fx.app, "work2").await?;
    press(&mut fx.app, KeyCode::Tab).await?;
    backspace(&mut fx.app, url.chars().count()).await?;
    type_text(&mut fx.app, "wss://fspec2.example.com").await?;
    press(&mut fx.app, KeyCode::Enter).await?;

    // @step Then the board keeps its connection
    assert_eq!(fx.connects.load(Ordering::SeqCst), 0);
    assert_eq!(fx.backend.destroy_session_calls(), 0);
    assert_eq!(
        saved_daemons::list(&fx.path)?,
        vec![saved("work2", "wss://fspec2.example.com", Some(TOKEN))]
    );

    // @step And the header reads "Daemon: work2 ▾"
    assert_eq!(fx.app.board_store().connected_daemon(), Some("work2"));

    // @step And a notice says the change applies the next time the TUI connects to it
    let screen = text(&render(&mut fx.app, 200, 40));
    assert!(screen.contains("Daemon: work2 ▾"), "{screen}");
    assert!(screen.contains("next time"), "{screen}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn removing_a_saved_daemon_asks_first() -> TestResult {
    // @step Given the switcher highlights saved daemon "work"
    let mut fx = fixture(Setup {
        saved: vec![saved("work", "wss://fspec.example.com", Some(TOKEN))],
        ..Setup::default()
    })
    .await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Down).await?;

    // @step When the user presses "d"
    press(&mut fx.app, KeyCode::Char('d')).await?;

    // @step Then the prompt reads "Remove saved daemon work and its stored token? y/n"
    assert!(fx.app.compositor().contains(CONFIRM));
    let screen = text(&render(&mut fx.app, 120, 30));
    assert!(
        screen.contains("Remove saved daemon work and its stored token? y/n"),
        "{screen}"
    );

    // @step And answering "n" keeps "work"
    press(&mut fx.app, KeyCode::Char('n')).await?;
    assert!(!fx.app.compositor().contains(CONFIRM));
    assert!(saved_daemons::find(&fx.path, "work")?.is_some());

    // @step And pressing "d" and answering "y" removes "work" and its token
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Down).await?;
    press(&mut fx.app, KeyCode::Char('d')).await?;
    press(&mut fx.app, KeyCode::Char('y')).await?;
    assert!(saved_daemons::list(&fx.path)?.is_empty());
    assert!(!std::fs::read_to_string(&fx.path)?.contains(TOKEN));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn local_daemons_cannot_be_edited_or_removed() -> TestResult {
    // @step Given the switcher highlights local daemon "alpha"
    let mut fx = fixture(Setup::default()).await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;

    // @step When the user presses "e" and then "d"
    press(&mut fx.app, KeyCode::Char('e')).await?;
    let edit = text(&render(&mut fx.app, 200, 30));
    let edit_form = fx.app.compositor().contains(FORM);
    press(&mut fx.app, KeyCode::Esc).await?;
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Char('d')).await?;
    let remove = text(&render(&mut fx.app, 200, 30));

    // @step Then both are refused because "alpha" is a local daemon
    assert!(!edit_form);
    for screen in [&edit, &remove] {
        assert!(screen.contains("alpha is a local daemon"), "{screen}");
    }

    // @step And no removal prompt is shown
    assert!(!fx.app.compositor().contains(CONFIRM));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn removing_the_connected_daemon_keeps_the_connection_and_its_sessions() -> TestResult {
    // @step Given the board is connected to saved daemon "work" at "wss://fspec.example.com" with an open session
    let mut fx = fixture(Setup {
        connected: "work".into(),
        saved: vec![saved("work", "wss://fspec.example.com", Some(TOKEN))],
        ..Setup::default()
    })
    .await?;
    fx.app
        .dispatch(Action::SessionCreated(SessionId::new("s-work")));
    settle(&mut fx.app).await?;
    fx.app.dispatch(Action::BackToBoard);
    settle(&mut fx.app).await?;

    // @step When the user removes "work" and confirms
    press(&mut fx.app, KeyCode::Char('s')).await?;
    press(&mut fx.app, KeyCode::Char('d')).await?;
    press(&mut fx.app, KeyCode::Char('y')).await?;

    // @step Then the header reads "Daemon: wss://fspec.example.com ▾"
    assert!(saved_daemons::list(&fx.path)?.is_empty());
    press(&mut fx.app, KeyCode::Esc).await?;
    let screen = text(&render(&mut fx.app, 160, 30));
    assert!(
        screen.contains("Daemon: wss://fspec.example.com ▾"),
        "{screen}"
    );

    // @step And no session was destroyed or interrupted and the session tab is still open
    assert_eq!(fx.backend.destroy_session_calls(), 0);
    assert_eq!(fx.backend.interrupt_calls(), 0);
    assert_eq!(fx.connects.load(Ordering::SeqCst), 0);
    assert!(fx
        .app
        .agent_view_store()
        .open_sessions()
        .iter()
        .any(|session| session.id == SessionId::new("s-work")));

    // @step And the switcher no longer lists "work"
    press(&mut fx.app, KeyCode::Char('s')).await?;
    assert!(!fx
        .app
        .board_store()
        .daemons()
        .iter()
        .any(|daemon| daemon.name == "work"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_switcher_and_the_dialog_fit_a_64_column_terminal() -> TestResult {
    // @step Given a 64-column terminal and a daemon with a 64-character name
    let long_name = format!("long-{}", "n".repeat(59));
    let mut fx = fixture(Setup {
        connected: long_name.clone(),
        saved: vec![saved(&long_name, "ws://127.0.0.1:4100", None)],
        ..Setup::default()
    })
    .await?;
    let header = render(&mut fx.app, 64, 24);

    // @step When the switcher is open and then the add dialog holds a 90-character URL
    press(&mut fx.app, KeyCode::Char('s')).await?;
    let switcher = render(&mut fx.app, 64, 24);
    press(&mut fx.app, KeyCode::Char('a')).await?;
    press(&mut fx.app, KeyCode::Tab).await?;
    let url = format!("wss://{}.example.com:8443", "h".repeat(67));
    assert_eq!(url.chars().count(), 90);
    type_text(&mut fx.app, &url).await?;
    let dialog = render(&mut fx.app, 64, 24);

    // @step Then the name is cut with "…" in the header and in the switcher
    let header_y = header_row(&header).or_fail("header row")?;
    let header_text = row(&header, header_y);
    assert!(header_text.contains("Daemon: long-"), "{header_text}");
    assert!(header_text.contains('…'), "{header_text}");
    assert!(!header_text.contains(&long_name), "{header_text}");
    let switcher_text = text(&switcher);
    let entry = switcher_text
        .lines()
        .find(|line| line.contains("long-nnn"))
        .or_fail("switcher entry")?;
    assert!(entry.contains('…'), "{switcher_text}");

    // @step And the dialog fits within 64 columns with all three fields visible
    let dialog_text = text(&dialog);
    for label in ["Name", "URL", "Token"] {
        assert!(dialog_text.contains(label), "{label}: {dialog_text}");
    }
    let url_y = find_row(&dialog, "URL").or_fail("URL row")?;
    let url_row = row(&dialog, url_y);

    // @step And the URL field shows the end of the URL after "…"
    assert!(url_row.contains('…'), "{url_row}");
    assert!(url_row.contains("example.com:8443"), "{url_row}");
    Ok(())
}
