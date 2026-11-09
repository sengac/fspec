use std::sync::Arc;

use codelet_fspec_tui::views::ProviderSettingsMode;
use codelet_fspec_tui::{Action, App};
use codelet_test_helpers::TestResult;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};

mod common;
use common::clipboard::{osc52, Clipboard, Launcher};
use common::{MockBackend, TaskOutcome};

const URL: &str = "https://claude.ai/oauth/authorize?code=true&state=focus-test";

struct Screen {
    app: App,
    backend: Arc<MockBackend>,
    clipboard: Clipboard,
    launcher: Launcher,
}

impl Screen {
    fn new() -> Self {
        let backend = Arc::new(MockBackend::new());
        let mut app = App::new(backend.clone());
        app.dispatch(Action::OpenProviderSettingsView);
        let clipboard = Clipboard::attach(&mut app);
        let launcher = Launcher::attach(&mut app);
        app.dispatch(Action::OAuthHeadlessReady {
            provider_id: "anthropic".into(),
            authorize_url: URL.into(),
            pkce_verifier: "verifier".into(),
            generation: app.navigator().provider_settings.oauth_generation(),
        });
        Self {
            app,
            backend,
            clipboard,
            launcher,
        }
    }

    async fn drain(&mut self) -> TestResult {
        loop {
            let mut progressed = false;
            while let Some(task) = self.app.next_pending_task() {
                task.await.completed_or_cancelled()?;
                progressed = true;
            }
            while let Some(action) = self.app.try_recv_action() {
                self.app.dispatch(action);
                progressed = true;
            }
            if !progressed {
                return Ok(());
            }
        }
    }

    async fn key(&mut self, code: KeyCode) -> TestResult {
        let _ = self
            .app
            .handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
        self.drain().await
    }

    async fn type_text(&mut self, text: &str) -> TestResult {
        for character in text.chars() {
            self.key(KeyCode::Char(character)).await?;
        }
        Ok(())
    }

    fn assert_entry(&self, expected: &str, focused: bool) {
        assert!(
            matches!(
                &self.app.navigator().provider_settings.mode,
                ProviderSettingsMode::OAuthHeadlessCodeEntry { code_input, code_focused, .. }
                    if code_input == expected && *code_focused == focused
            ),
            "expected code {expected:?}, focused={focused}"
        );
    }

    fn rendered(&mut self) -> String {
        let area = Rect::new(0, 0, 120, 30);
        let mut buffer = Buffer::empty(area);
        self.app.render(area, &mut buffer);
        (0..30)
            .map(|row| {
                (0..120)
                    .map(|column| buffer[(column, row)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn assert_submitted(&self, code: &str) {
        assert_eq!(
            self.backend.oauth_headless_complete_args(),
            vec![("anthropic".into(), code.into(), "verifier".into())]
        );
    }
}

#[tokio::test(start_paused = true)]
async fn copy_then_open_keeps_url_focus_and_leaves_the_code_empty() -> TestResult {
    // @step Given sign in with code has URL focus
    let mut screen = Screen::new();
    // @step When I press c then o
    screen.type_text("co").await?;
    // @step Then the URL is copied and opened
    assert_eq!(screen.clipboard.bytes()?, osc52(URL));
    assert_eq!(screen.launcher.opened()?, vec![URL]);
    // @step And URL focus remains with an empty code and URL action hints
    screen.assert_entry("", false);
    let rendered = screen.rendered();
    assert!(rendered.contains("c: copy URL"));
    assert!(rendered.contains("o: open URL"));
    // @step And Enter does not submit a code
    screen.key(KeyCode::Enter).await?;
    assert!(screen.backend.oauth_headless_complete_args().is_empty());
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn failed_open_then_copy_follows_the_recovery_hint_without_typing_c() -> TestResult {
    // @step Given sign in with code has URL focus and the browser launcher fails
    let mut screen = Screen::new();
    screen.launcher.fail_with("no browser")?;
    // @step When I press o then c
    screen.type_text("oc").await?;
    // @step Then the failure explains how to copy and the URL is copied
    assert_eq!(screen.launcher.opened()?, vec![URL]);
    assert!(screen
        .rendered()
        .contains("Could not open a browser: no browser. Press c to copy the URL."));
    assert_eq!(screen.clipboard.bytes()?, osc52(URL));
    // @step And URL focus remains with an empty code
    screen.assert_entry("", false);
    // @step And Enter does not submit a code
    screen.key(KeyCode::Enter).await?;
    assert!(screen.backend.oauth_headless_complete_args().is_empty());
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn explicit_field_focus_preserves_codes_starting_with_c_or_o_after_url_actions() -> TestResult
{
    for focus in [KeyCode::Tab, KeyCode::Down] {
        for code in ["co9#st", "oc9#st"] {
            // @step Given sign in with code has URL focus
            let mut screen = Screen::new();
            // @step When I copy the URL and focus the Code field with Tab or Down
            screen.type_text("c").await?;
            screen.key(focus).await?;
            // @step And I type a code starting with c or o and press Enter
            screen.type_text(code).await?;
            screen.assert_entry(code, true);
            screen.key(KeyCode::Enter).await?;
            // @step Then exactly the typed code is submitted without opening a URL
            screen.assert_submitted(code);
            assert!(screen.launcher.opened()?.is_empty());
        }
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn pasting_after_url_actions_preserves_codes_starting_with_c_or_o() -> TestResult {
    for code in ["co9#st", "oc9#st"] {
        // @step Given sign in with code has URL focus
        let mut screen = Screen::new();
        // @step When I copy and open the URL then paste a code starting with c or o
        screen.type_text("co").await?;
        let _ = screen.app.handle_paste(&format!(" {code}\n"));
        screen.drain().await?;
        // @step Then the paste focuses the Code field and preserves the code
        screen.assert_entry(code, true);
        assert_eq!(screen.clipboard.bytes()?, osc52(URL));
        assert_eq!(screen.launcher.opened()?, vec![URL]);
        // @step And Enter submits exactly the pasted code
        screen.key(KeyCode::Enter).await?;
        screen.assert_submitted(code);
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn other_typing_enters_code_and_returning_to_the_url_preserves_it() -> TestResult {
    // @step Given sign in with code has URL focus
    let mut screen = Screen::new();
    // @step When I copy the URL and type a code starting with a different character
    screen.type_text("cabc#state").await?;
    screen.assert_entry("abc#state", true);
    // @step And I return to the URL with Up and press c then o
    screen.key(KeyCode::Up).await?;
    screen.type_text("co").await?;
    // @step Then the URL is copied and opened without changing the code
    assert_eq!(screen.clipboard.bytes()?, [osc52(URL), osc52(URL)].concat());
    assert_eq!(screen.launcher.opened()?, vec![URL]);
    screen.assert_entry("abc#state", false);
    // @step And Down returns to the Code field where c and o are typed literally
    screen.key(KeyCode::Down).await?;
    screen.type_text("co").await?;
    screen.assert_entry("abc#stateco", true);
    Ok(())
}
