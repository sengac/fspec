//! TUI-126 — the Provider Settings sign-in screens copy, open, paste and wrap
//! their text.
//!
//! Feature: spec/features/sign-in-screens-copy-open-paste-and-wrap.feature
//!
//! Every test drives the real `App` over a `MockBackend` from the key or paste
//! event to its effect: the OSC 52 bytes the App writes to its clipboard
//! writer, the URL the App's launcher is asked to open, the code the App
//! submits, and the rows `App::render` paints. Time is tokio's paused clock.

use std::sync::Arc;

use codelet_fspec_tui::views::ProviderSettingsMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::clipboard::{osc52, Clipboard, Launcher};
use common::MockBackend;
use common::TaskOutcome;

/// A real-shaped Claude authorize URL (384 characters).
fn authorize_url() -> String {
    let url = format!(
        "https://claude.ai/oauth/authorize?code=true&client_id={}&response_type=code\
         &redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
        "https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback%3Fsource%3Dfspec",
        "org%3Acreate_api_key+user%3Aprofile+user%3Ainference+user%3Asessions",
        "Q2hhbGxlbmdlQ2hhbGxlbmdlQ2hhbGxlbmdlQ2hhbGxlbmdlQ2hh",
        "c3RhdGVzdGF0ZXN0YXRlc3RhdGVzdGF0ZXN0YXRlc3RhdGVzdGF0ZXN0YXRlc3RhdGU0",
    );
    // Cut or pad the state to exactly 384 characters.
    let mut url: String = url.chars().take(384).collect();
    while url.len() < 384 {
        url.push('x');
    }
    url
}

struct Screen {
    app: App,
    mock: Arc<MockBackend>,
    clipboard: Clipboard,
    launcher: Launcher,
}

/// Provider Settings open, with the clipboard and the URL launcher captured.
fn provider_settings() -> Screen {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::OpenProviderSettingsView);
    let clipboard = Clipboard::attach(&mut app);
    let launcher = Launcher::attach(&mut app);
    Screen {
        app,
        mock,
        clipboard,
        launcher,
    }
}

/// Anthropic sign in with code, reached the way the start answer arrives.
fn code_entry(url: &str) -> Screen {
    let mut screen = provider_settings();
    let generation = screen.app.navigator().provider_settings.oauth_generation();
    screen.app.dispatch(Action::OAuthHeadlessReady {
        provider_id: "anthropic".into(),
        authorize_url: url.to_string(),
        pkce_verifier: "verifier".into(),
        generation,
    });
    screen
}

fn set_mode(screen: &mut Screen, mode: ProviderSettingsMode) {
    screen.app.navigator_mut().provider_settings.mode = mode;
}

/// Run every parked task and queued action until the App is quiet.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..50 {
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

async fn key(screen: &mut Screen, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    let _ = screen
        .app
        .handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(&mut screen.app).await
}

async fn press(screen: &mut Screen, ch: char) -> TestResult {
    key(screen, KeyCode::Char(ch), KeyModifiers::NONE).await
}

async fn ctrl_c(screen: &mut Screen) -> TestResult {
    key(screen, KeyCode::Char('c'), KeyModifiers::CONTROL).await
}

async fn type_text(screen: &mut Screen, text: &str) -> TestResult {
    for ch in text.chars() {
        press(screen, ch).await?;
    }
    Ok(())
}

async fn paste(screen: &mut Screen, text: &str) -> TestResult {
    let _ = screen.app.handle_paste(text);
    drain(&mut screen.app).await
}

/// The last OSC 52 copy the App wrote, compared with `text`.
fn copied(screen: &Screen, text: &str) -> Result<bool, TestError> {
    Ok(screen.clipboard.bytes()?.ends_with(&osc52(text)))
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

/// The rows from the one reading `label` down to the next blank row, joined
/// with their trailing padding trimmed.
fn rows_under(rows: &[String], label: &str) -> Option<String> {
    let start = rows.iter().position(|row| row.trim_end() == label)?;
    Some(
        rows[start + 1..]
            .iter()
            .map(|row| row.trim_end())
            .take_while(|row| !row.is_empty())
            .collect::<String>(),
    )
}

/// The whole of `text` appears on screen, possibly split across rows.
fn shown_whole(rows: &[String], text: &str) -> bool {
    let joined: String = rows.iter().map(|row| row.trim_end()).collect();
    joined.contains(text)
}

fn code_input(app: &App) -> Option<String> {
    match &app.navigator().provider_settings.mode {
        ProviderSettingsMode::OAuthHeadlessCodeEntry { code_input, .. } => Some(code_input.clone()),
        _ => None,
    }
}

#[tokio::test(start_paused = true)]
async fn c_copies_the_whole_authorize_url_and_the_copy_is_confirmed() -> TestResult {
    // @step Given Anthropic sign in with code shows a 384-character authorize URL
    let url = authorize_url();
    assert_eq!(url.len(), 384);
    let mut screen = code_entry(&url);

    // @step When I press c
    press(&mut screen, 'c').await?;

    // @step Then the clipboard receives exactly the 384-character URL
    assert_eq!(screen.clipboard.bytes()?, osc52(&url));
    // @step And the screen confirms "Sent 384 characters to the terminal clipboard (OSC 52)"
    assert!(screen_text(&mut screen.app, 120, 30)
        .contains("Sent 384 characters to the terminal clipboard (OSC 52)"));
    // @step And the copy confirmation is gone 2 seconds later
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    assert!(!screen_text(&mut screen.app, 120, 30)
        .contains("Sent 384 characters to the terminal clipboard (OSC 52)"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn o_opens_the_authorize_url_and_a_failed_open_says_how_to_copy_it() -> TestResult {
    // @step Given Anthropic sign in with code shows the authorize URL
    let url = authorize_url();
    let mut screen = code_entry(&url);

    // @step When I press o
    press(&mut screen, 'o').await?;

    // @step Then the URL launcher is asked to open the authorize URL
    assert_eq!(screen.launcher.opened()?, vec![url.clone()]);

    // @step When I press o again and the launcher fails with "no browser"
    screen.launcher.fail_with("no browser")?;
    press(&mut screen, 'o').await?;
    assert_eq!(screen.launcher.opened()?.len(), 2);

    // @step Then the screen shows "Could not open a browser: no browser. Press c to copy the URL."
    let rows = rows(&mut screen.app, 120, 30);
    assert!(shown_whole(
        &rows,
        "Could not open a browser: no browser. Press c to copy the URL."
    ));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_code_that_starts_with_c_or_o_keeps_its_first_character() -> TestResult {
    // @step Given Anthropic sign in with code shows the authorize URL
    let url = authorize_url();
    let mut screen = code_entry(&url);

    // @step When I press c to copy the URL
    press(&mut screen, 'c').await?;
    assert!(copied(&screen, &url)?);
    // @step And I press Tab to focus the Code field
    key(&mut screen, KeyCode::Tab, KeyModifiers::NONE).await?;
    // @step And I type "co9#st"
    type_text(&mut screen, "co9#st").await?;

    // @step Then the Code field holds "co9#st"
    assert_eq!(code_input(&screen.app).as_deref(), Some("co9#st"));
    assert_eq!(screen.launcher.opened()?, Vec::<String>::new());

    // @step When I press Enter
    key(&mut screen, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then the code "co9#st" is submitted
    assert_eq!(
        screen.mock.oauth_headless_complete_args(),
        vec![("anthropic".into(), "co9#st".into(), "verifier".into())]
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_pasted_code_goes_into_the_code_field_without_its_line_break() -> TestResult {
    // @step Given Anthropic sign in with code shows the authorize URL
    let mut screen = code_entry(&authorize_url());

    // @step When I paste "abc#xyz" followed by a line break
    paste(&mut screen, "abc#xyz\n").await?;

    // @step Then the Code field holds "abc#xyz"
    assert_eq!(code_input(&screen.app).as_deref(), Some("abc#xyz"));

    // @step When I press Enter
    key(&mut screen, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then the code "abc#xyz" is submitted
    assert_eq!(
        screen.mock.oauth_headless_complete_args(),
        vec![("anthropic".into(), "abc#xyz".into(), "verifier".into())]
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_authorize_url_and_a_long_code_wrap_at_80_and_at_120_columns() -> TestResult {
    // @step Given Anthropic sign in with code shows a 384-character authorize URL
    let url = authorize_url();
    let mut screen = code_entry(&url);
    // @step And I pasted a 90-character code
    let code = format!("{}#{}", "k".repeat(60), "s".repeat(29));
    assert_eq!(code.len(), 90);
    paste(&mut screen, &code).await?;

    // @step When the screen is drawn 80 columns wide and again 120 columns wide
    for (width, height) in [(80, 24), (120, 30)] {
        let rows = rows(&mut screen.app, width, height);
        // @step Then each time the rows under "Visit:" hold the whole URL and nothing else
        assert_eq!(rows_under(&rows, "Visit:").as_deref(), Some(url.as_str()));
        // @step And each time the whole code is on screen
        assert!(shown_whole(&rows, &code), "code cut off at {width} columns");
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn ctrl_c_copies_the_focused_url_or_the_typed_code() -> TestResult {
    // @step Given Anthropic sign in with code shows the authorize URL
    let url = authorize_url();
    let mut screen = code_entry(&url);

    // @step When I press Ctrl+C
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives the authorize URL
    assert_eq!(screen.clipboard.bytes()?, osc52(&url));

    // @step When I press Tab, type "abc" and press Ctrl+C
    key(&mut screen, KeyCode::Tab, KeyModifiers::NONE).await?;
    type_text(&mut screen, "abc").await?;
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives "abc"
    assert!(copied(&screen, "abc")?);
    assert_eq!(code_input(&screen.app).as_deref(), Some("abc"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn device_login_copies_the_user_code_and_opens_the_verification_url() -> TestResult {
    // @step Given Codex device login shows the user code "ABCD-1234" and a verification URL
    let mut screen = provider_settings();
    let verification = "https://auth.openai.com/codex/device";
    set_mode(
        &mut screen,
        ProviderSettingsMode::OAuthDeviceWaiting {
            provider_id: "codex".into(),
            user_code: "ABCD-1234".into(),
            verification_url: verification.into(),
        },
    );

    // @step When I press c
    press(&mut screen, 'c').await?;

    // @step Then the clipboard receives "ABCD-1234"
    assert_eq!(screen.clipboard.bytes()?, osc52("ABCD-1234"));

    // @step When I press Ctrl+C
    screen.clipboard.clear()?;
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives "ABCD-1234" again
    assert_eq!(screen.clipboard.bytes()?, osc52("ABCD-1234"));

    // @step When I press o
    press(&mut screen, 'o').await?;

    // @step Then the URL launcher is asked to open the verification URL
    assert_eq!(screen.launcher.opened()?, vec![verification.to_string()]);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn device_login_waiting_for_its_code_says_so_and_copies_nothing() -> TestResult {
    // @step Given GitHub Copilot device login is waiting for its device code
    let mut screen = provider_settings();
    set_mode(
        &mut screen,
        ProviderSettingsMode::OAuthDeviceWaiting {
            provider_id: "github-copilot".into(),
            user_code: String::new(),
            verification_url: String::new(),
        },
    );

    // @step When the screen is drawn
    let text = screen_text(&mut screen.app, 80, 24);

    // @step Then it shows "Requesting device code…"
    assert!(text.contains("Requesting device code…"));
    assert!(!text.contains("Your code:"));

    // @step When I press c
    press(&mut screen, 'c').await?;
    ctrl_c(&mut screen).await?;
    press(&mut screen, 'o').await?;

    // @step Then nothing is copied
    assert_eq!(screen.clipboard.bytes()?, Vec::<u8>::new());
    assert_eq!(screen.launcher.opened()?, Vec::<String>::new());
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_long_oauth_error_is_shown_whole_and_can_be_copied() -> TestResult {
    // @step Given the sign-in failed with a 250-character error
    let mut screen = code_entry(&authorize_url());
    let error = format!(
        "token exchange failed: HTTP 400 Bad Request: {}",
        "invalid_grant the authorization code has expired or was already used ".repeat(3)
    )
    .chars()
    .take(250)
    .collect::<String>();
    assert_eq!(error.chars().count(), 250);
    let generation = screen.app.navigator().provider_settings.oauth_generation();
    screen.app.dispatch(Action::OAuthLoginFailed {
        provider_id: "anthropic".into(),
        error: error.clone(),
        generation,
    });

    // @step When the screen is drawn 80 columns wide
    let rows = rows(&mut screen.app, 80, 24);

    // @step Then the whole error is on screen
    let joined: String = rows
        .iter()
        .map(|row| row.trim_end())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(joined.contains(error.trim_end()), "error cut off: {joined}");

    // @step When I press c
    press(&mut screen, 'c').await?;

    // @step Then the clipboard receives the error text
    assert_eq!(screen.clipboard.bytes()?, osc52(&error));

    // @step When I press Ctrl+C
    screen.clipboard.clear()?;
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives the error text again
    assert_eq!(screen.clipboard.bytes()?, osc52(&error));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_github_enterprise_host_field_takes_a_paste_and_copies_it() -> TestResult {
    // @step Given the GitHub Enterprise host field shows a validation error
    let mut screen = provider_settings();
    set_mode(
        &mut screen,
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            provider_id: "github-copilot".into(),
            url_input: String::new(),
            validation_error: Some("Enter a GitHub Enterprise host".into()),
        },
    );

    // @step When I paste "ghe.example.com"
    paste(&mut screen, "ghe.example.com").await?;

    // @step Then the field holds "ghe.example.com" and the validation error is gone
    assert_eq!(
        screen.app.navigator().provider_settings.mode,
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            provider_id: "github-copilot".into(),
            url_input: "ghe.example.com".into(),
            validation_error: None,
        }
    );

    // @step When I press Ctrl+C
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives "ghe.example.com"
    assert_eq!(screen.clipboard.bytes()?, osc52("ghe.example.com"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_browser_waiting_screen_says_what_to_do_when_no_browser_opens() -> TestResult {
    // @step Given Anthropic browser sign-in is waiting for authorization
    let mut screen = provider_settings();
    set_mode(
        &mut screen,
        ProviderSettingsMode::OAuthBrowserWaiting {
            provider_id: "anthropic".into(),
        },
    );

    // @step When the screen is drawn
    let rows = rows(&mut screen.app, 80, 24);

    // @step Then it shows "Browser didn't open? Press Esc and use Sign in with code."
    assert!(shown_whole(
        &rows,
        "Browser didn't open? Press Esc and use Sign in with code."
    ));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_long_list_status_wraps_and_ctrl_c_copies_it() -> TestResult {
    // @step Given Provider Settings shows a 200-character status in List mode
    let mut screen = provider_settings();
    let status = format!("✗ {}", "list failed: connection refused ".repeat(7))
        .chars()
        .take(200)
        .collect::<String>();
    screen
        .app
        .dispatch(Action::ProviderSettingsStatus(status.clone()));
    assert_eq!(
        screen.app.navigator().provider_settings.mode,
        ProviderSettingsMode::List
    );

    // @step When the screen is drawn 80 columns wide
    let rows = rows(&mut screen.app, 80, 24);

    // @step Then the whole status is on screen over at most 3 rows
    let first = rows
        .iter()
        .position(|row| row.contains("✗ list failed"))
        .or_fail("the status is painted")?;
    let joined = rows[first..(first + 3).min(rows.len())]
        .iter()
        .map(|row| row.trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        joined.contains(status.trim_end()),
        "status cut off: {joined}"
    );

    // @step When I press Ctrl+C
    ctrl_c(&mut screen).await?;

    // @step Then the clipboard receives the status
    assert_eq!(screen.clipboard.bytes()?, osc52(&status));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_hints_follow_the_focus_on_the_code_entry_screen() -> TestResult {
    // @step Given Anthropic sign in with code shows the authorize URL
    let mut screen = code_entry(&authorize_url());

    // @step When the screen is drawn
    let text = screen_text(&mut screen.app, 120, 30);

    // @step Then the hints offer "c: copy URL" and "o: open URL"
    assert!(text.contains("c: copy URL"));
    assert!(text.contains("o: open URL"));

    // @step When I press Tab to focus the Code field
    key(&mut screen, KeyCode::Tab, KeyModifiers::NONE).await?;

    // @step Then the hints no longer offer c or o
    let text = screen_text(&mut screen.app, 120, 30);
    assert!(!text.contains("c: copy"));
    assert!(!text.contains("o: open"));
    Ok(())
}
