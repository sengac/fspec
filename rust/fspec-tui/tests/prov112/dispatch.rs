//! PROV-112 section 2: App::dispatch of disconnect against the shared MockBackend.

use super::*;
use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;

// ════════════════════════ SECTION 2: DISPATCH ════════════════════════

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// A connected OAuth provider credential row (configured + no env masked key
/// → the projection classifies it as OAuth-logged-in and emits the Logout
/// row).
fn oauth_pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: "oauth".to_string(),
        model_count: 0,
        masked_key: None,
        source: Some(id.to_string()),
    }
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

async fn wait_until<F: FnMut() -> bool>(
    mut predicate: F,
    label: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    // TEST-042: poll between yields under the deadlock guard.
    codelet_test_helpers::wait_until(label, || std::future::ready(predicate())).await?;
    Ok(())
}

/// BUG-334: a credential-shaped secret the scripted backend error carries.
const LEAKED_TOKEN: &str = "ghr_bug334_secret_refresh_value";

/// BUG-334: render the Provider Settings view the way the TUI paints it.
fn render_provider_settings(app: &mut App) -> String {
    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    app.navigator_mut().provider_settings.render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Confirming disconnect clears codex tokens once and refreshes the
// nav (dispatch half — the y-press → OAuthDisconnect emission is covered in
// the view test).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_clears_codex_tokens_once_and_refreshes() -> TestResult {
    // @step Given a provider "codex" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo("codex")]);
    let mut app = fresh_app(mock.clone());

    // @step And the cursor is on the "Logout from OAuth [ChatGPT]" row
    // @step And the user has opened the DisconnectOAuth confirm
    // (the dispatch test enters at the OAuthDisconnect action; the view test
    // covers the y-press that emits it)

    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "codex".to_string(),
    });
    drain_pending(&mut app).await?;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens called at least once",
    )
    .await?;

    // @step Then the backend codex clear-tokens method is called exactly once
    assert_eq!(mock.oauth_clear_tokens_calls(), 1);
    assert_eq!(
        mock.oauth_clear_tokens_providers(),
        vec!["codex".to_string()]
    );

    // @step And the cached OPENAI_API_KEY is preserved
    // (the codex clear strips ONLY the tokens field — proven by the pure
    // `strip_codex_tokens` unit test in codelet-rpc; the backend boundary here
    // never deletes a key)

    // @step And the credentials are re-fetched through the numbered provider settings refresh
    assert!(mock.list_provider_credentials_calls() >= 1);

    // @step And the "Logout from OAuth [ChatGPT]" row is gone
    let view = &app.navigator().provider_settings;
    let has_logout = view.nav_items.iter().any(|i| {
        matches!(
            i.kind,
            codelet_fspec_tui::views::NavItemKind::OAuthStatus { .. }
        )
    });
    assert!(
        !has_logout,
        "the Logout row must disappear after disconnect"
    );

    // @step And the cursor returns to the "codex" provider row
    // (cursor lands on the codex provider row; with the Logout row gone the
    // only codex row left is the provider header)
    if let Some(item) = view.focused_nav_item() {
        assert_eq!(item.provider_id, "codex");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A backend clear error returns to list silently without leaking
// the RPC name
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn backend_clear_error_returns_to_list_without_leaking_rpc_name() -> TestResult {
    // @step Given a provider "github-copilot" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo("github-copilot")]);
    // @step And the backend clear-credential method is scripted to return an error
    mock.set_oauth_clear_tokens_error(format!(
        "copilot_oauth_clear_credential failed: refresh_token={LEAKED_TOKEN}"
    ));
    let mut app = fresh_app(mock.clone());
    app.dispatch(Action::SessionCreated(sid("s-1")));
    drain_pending(&mut app).await?;

    // @step And the user has opened the DisconnectOAuth confirm
    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "github-copilot".to_string(),
    });
    drain_pending(&mut app).await?;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens attempted",
    )
    .await?;

    // @step Then the UI returns to list
    assert!(matches!(
        app.navigator().provider_settings.mode,
        codelet_fspec_tui::views::ProviderSettingsMode::List
    ));

    // @step And a clear logout failure is shown without credential contents
    let screen = render_provider_settings(&mut app);
    assert!(
        screen.contains("OAuth logout failed"),
        "the List view must show the logout failure, got:\n{screen}"
    );
    assert!(
        !screen.contains(LEAKED_TOKEN) && !screen.contains("refresh_token"),
        "the failure must not show credential contents, got:\n{screen}"
    );

    // @step And no RPC or method name is shown anywhere in the UI
    assert!(
        !screen.contains("oauth_clear_tokens") && !screen.contains("clear_credential"),
        "the rendered view must not leak the RPC/method name, got:\n{screen}"
    );
    let status = &app.navigator().provider_settings.status;
    assert!(
        !status.contains("oauth_clear_tokens")
            && !status.contains("copilot_oauth_clear_credential")
            && !status.contains("clear_credential"),
        "status must not leak the RPC/method name, got {status:?}"
    );
    // And no leak into the agent scrollback either.
    if let Some(ctx) = app.agent_view_store().session_context_for(&sid("s-1")) {
        let text = ctx
            .scrollback
            .visible_window(1024)
            .iter()
            .flat_map(|c| {
                c.lines.iter().map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
            })
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            !text.contains("oauth_clear_tokens") && !text.contains("clear_credential"),
            "scrollback must not leak RPC method names, got {text:?}"
        );
    }

    // @step And the clear operation is idempotent on a subsequent reload
    // (a second disconnect still resolves cleanly and silently — even with the
    // error still scripted, the dispatch swallows it and re-attempts)
    app.dispatch(Action::OAuthDisconnect {
        provider_id: "github-copilot".to_string(),
    });
    drain_pending(&mut app).await?;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 2,
        "second clear attempted (idempotent)",
    )
    .await?;
    assert_eq!(mock.oauth_clear_tokens_calls(), 2);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario Outline: Disconnect routes to the correct per-provider clear
// method (dispatch half — the backend records the provider_id it was asked to
// clear exactly once; the per-provider file routing is proven inside
// codelet-rpc's oauth_disconnect dispatch + strip_codex_tokens unit test).
// ─────────────────────────────────────────────────────────────────────────

async fn assert_clear_called_once_for(
    provider: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    // @step Given a provider "<provider>" is expanded with OAuth tokens present
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![oauth_pinfo(provider)]);
    let mut app = fresh_app(mock.clone());

    // @step And the user has opened the DisconnectOAuth confirm
    // @step When the user presses "y"
    app.dispatch(Action::OAuthDisconnect {
        provider_id: provider.to_string(),
    });
    drain_pending(&mut app).await?;
    wait_until(
        || mock.oauth_clear_tokens_calls() >= 1,
        "oauth_clear_tokens called",
    )
    .await?;

    // @step Then the backend "<clear_method>" is called exactly once for provider "<provider>"
    assert_eq!(mock.oauth_clear_tokens_calls(), 1);
    assert_eq!(
        mock.oauth_clear_tokens_providers(),
        vec![provider.to_string()],
        "clear must be routed for {provider} exactly once"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_anthropic() -> TestResult {
    assert_clear_called_once_for("anthropic").await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_codex() -> TestResult {
    assert_clear_called_once_for("codex").await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_routes_clear_per_provider_github_copilot() -> TestResult {
    assert_clear_called_once_for("github-copilot").await?;
    Ok(())
}
