//! PROV-098 — Provider Settings nav tree shows credential status.
//!
//! Feature: spec/features/provider-settings-nav-tree-credential-status.feature
//!
//! The rich RPC-103/349 nav tree dropped credential-display data. These
//! tests pin BOTH layers:
//!   * the pure `project_display_infos` projection — env api-key
//!     passthrough AND the synthetic OAuth display
//!     (`masked_key = "OAuth"`, `source = <oauth label>`); and
//!   * the render layer (`row_kind_and_label` via `render`) — provider +
//!     ApiKey rows carry `✓ {masked} [{source}]` / `(not configured)` /
//!     `(not set)` exactly mirroring the TS `ProviderSettingsPanel`.
//!
//! Pure render/projection tests over hand-built `ProviderDisplayInfo` /
//! `ProviderCredentialInfo` records — no env, no filesystem, no network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::nav_item::ProviderDisplayInfo;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_rpc_types::ProviderCredentialInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

/// Build a non-oauth api-key ProviderDisplayInfo with the given display
/// masked_key / source. `requires_api_key` is true so an ApiKey child row
/// appears on expansion (build_nav_items gating).
fn api_key_display(
    id: &str,
    name: &str,
    masked_key: Option<&str>,
    source: Option<&str>,
) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: name.to_string(),
        configured: masked_key.is_some(),
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: true,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: masked_key.map(ToString::to_string),
        source: source.map(ToString::to_string),
    }
}

/// Build an OAuth-logged-in backend credential record (env api key absent,
/// so backend masked_key is None but the provider is configured).
fn oauth_credential(id: &str, display_name: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: display_name.to_string(),
        configured: true,
        credential_type: "oauth".to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

fn area() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    }
}

/// Render a view loaded with the supplied display infos into a buffer.
fn render_view(infos: Vec<ProviderDisplayInfo>) -> Buffer {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(infos);
    let a = area();
    let mut buf = Buffer::empty(a);
    view.render(a, &mut buf);
    buf
}

/// Render a view loaded with the supplied display infos, expanding the
/// given provider id first.
fn render_view_expanded(infos: Vec<ProviderDisplayInfo>, expand: &str) -> Buffer {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(infos);
    view.toggle_expansion(expand);
    let a = area();
    let mut buf = Buffer::empty(a);
    view.render(a, &mut buf);
    buf
}

/// Concatenate the visible symbols of row `y` into a String.
fn row_string(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

/// True when any row in `buf` contains `needle` as a substring.
fn any_row_contains(buf: &Buffer, needle: &str) -> bool {
    (0..buf.area.height).any(|y| row_string(buf, y).contains(needle))
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: ProviderDisplayInfo defaults masked_key and source to None
// ────────────────────────────────────────────────────────────────────────

#[test]
fn provider_display_info_defaults_masked_key_and_source_to_none() {
    // @step Given a default-constructed ProviderDisplayInfo
    let info = ProviderDisplayInfo::default();

    // @step Then its masked_key field is None
    assert!(info.masked_key.is_none(), "default masked_key must be None");

    // @step And its source field is None
    assert!(info.source.is_none(), "default source must be None");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Projection copies env api-key masked_key and source verbatim
// ────────────────────────────────────────────────────────────────────────

#[test]
fn projection_copies_env_api_key_masked_key_and_source_verbatim() {
    // @step Given a ProviderCredentialInfo for "openai" whose masked_key is Some "sk-••••••••cdef" and source is Some "env"
    let info = ProviderCredentialInfo {
        provider_id: "openai".to_string(),
        display_name: "OpenAI API".to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 4,
        masked_key: Some("sk-••••••••cdef".to_string()),
        source: Some("env".to_string()),
    };

    // @step When project_display_infos projects the credential list
    let display = project_display_infos(&[info], &[]);
    let openai = display
        .iter()
        .find(|d| d.id == "openai")
        .expect("openai display info present");

    // @step Then the resulting openai ProviderDisplayInfo masked_key is Some "sk-••••••••cdef"
    assert_eq!(openai.masked_key.as_deref(), Some("sk-••••••••cdef"));

    // @step And the resulting openai ProviderDisplayInfo source is Some "env"
    assert_eq!(openai.source.as_deref(), Some("env"));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Projection synthesizes OAuth display masked_key and source
// ────────────────────────────────────────────────────────────────────────

#[test]
fn projection_synthesizes_oauth_display_for_logged_in_provider() {
    // @step Given a ProviderCredentialInfo for "codex" of credential type "oauth" that is configured with masked_key None
    let info = oauth_credential("codex", "Codex (ChatGPT)");

    // @step When project_display_infos projects the credential list
    let display = project_display_infos(&[info], &[]);
    let codex = display
        .iter()
        .find(|d| d.id == "codex")
        .expect("codex display info present");

    // @step Then the resulting codex ProviderDisplayInfo masked_key is Some "OAuth"
    assert_eq!(codex.masked_key.as_deref(), Some("OAuth"));

    // @step And the resulting codex ProviderDisplayInfo source is Some "ChatGPT"
    assert_eq!(codex.source.as_deref(), Some("ChatGPT"));

    // @step And the resulting codex ProviderDisplayInfo has_oauth_tokens is true
    assert!(
        codex.has_oauth_tokens,
        "codex OAuth login (no env key) must remain OAuth-logged-in"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Configured provider row shows checkmark, masked key and source tag
// ────────────────────────────────────────────────────────────────────────

#[test]
fn configured_provider_row_shows_check_masked_key_and_source() {
    // @step Given a ProviderSettings view loaded with an "openai" provider whose masked_key is Some "sk-••••••••cdef" and source is Some "env"
    let infos = vec![api_key_display(
        "openai",
        "OpenAI API",
        Some("sk-••••••••cdef"),
        Some("env"),
    )];

    // @step When the nav tree is rendered into a buffer
    let buf = render_view(infos);

    // @step Then the openai provider row contains "OpenAI API ✓ sk-••••••••cdef [env]"
    assert!(
        any_row_contains(&buf, "OpenAI API ✓ sk-••••••••cdef [env]"),
        "expected configured provider annotation"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Configured provider row with no source omits the bracket tag
// ────────────────────────────────────────────────────────────────────────

#[test]
fn configured_provider_row_with_no_source_omits_bracket_tag() {
    // @step Given a ProviderSettings view loaded with an "openai" provider whose masked_key is Some "sk-••••••••cdef" and source is None
    let infos = vec![api_key_display(
        "openai",
        "OpenAI API",
        Some("sk-••••••••cdef"),
        None,
    )];

    // @step When the nav tree is rendered into a buffer
    let buf = render_view(infos);

    // @step Then the openai provider row contains "OpenAI API ✓ sk-••••••••cdef"
    let y = (0..buf.area.height)
        .find(|&y| row_string(&buf, y).contains("OpenAI API ✓ sk-••••••••cdef"))
        .expect("openai row present");

    // @step And the openai provider row does not contain "["
    assert!(
        !row_string(&buf, y).contains('['),
        "no source ⇒ no bracket tag; row was {:?}",
        row_string(&buf, y)
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Unconfigured provider row shows the not-configured annotation
// ────────────────────────────────────────────────────────────────────────

#[test]
fn unconfigured_provider_row_shows_not_configured() {
    // @step Given a ProviderSettings view loaded with a "cohere" provider whose masked_key is None
    let infos = vec![api_key_display("cohere", "Cohere", None, None)];

    // @step When the nav tree is rendered into a buffer
    let buf = render_view(infos);

    // @step Then the cohere provider row contains "Cohere (not configured)"
    assert!(
        any_row_contains(&buf, "Cohere (not configured)"),
        "expected unconfigured provider annotation"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Configured ApiKey child row shows checkmark, masked key and source tag
// ────────────────────────────────────────────────────────────────────────

#[test]
fn configured_api_key_child_row_shows_check_masked_key_and_source() {
    // @step Given a ProviderSettings view loaded with a "gemini" provider whose masked_key is Some "AIza••••••••H3Ck" and source is Some "env"
    let infos = vec![api_key_display(
        "gemini",
        "Google Gemini",
        Some("AIza••••••••H3Ck"),
        Some("env"),
    )];

    // @step And the "gemini" provider row is expanded
    // @step When the nav tree is rendered into a buffer
    let buf = render_view_expanded(infos, "gemini");

    // @step Then the gemini ApiKey child row contains "API Key ✓ AIza••••••••H3Ck [env]"
    assert!(
        any_row_contains(&buf, "API Key ✓ AIza••••••••H3Ck [env]"),
        "expected configured ApiKey child annotation"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Unconfigured ApiKey child row shows the not-set annotation
// ────────────────────────────────────────────────────────────────────────

#[test]
fn unconfigured_api_key_child_row_shows_not_set() {
    // @step Given a ProviderSettings view loaded with a "gemini" provider whose masked_key is None
    let infos = vec![api_key_display("gemini", "Google Gemini", None, None)];

    // @step And the "gemini" provider row is expanded
    // @step When the nav tree is rendered into a buffer
    let buf = render_view_expanded(infos, "gemini");

    // @step Then the gemini ApiKey child row contains "API Key (not set)"
    assert!(
        any_row_contains(&buf, "API Key (not set)"),
        "expected unconfigured ApiKey child annotation"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: OAuth-logged-in provider header shows synthetic OAuth annotation
//           plus a separate logout row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn oauth_logged_in_provider_header_shows_oauth_annotation_and_logout_row() {
    // @step Given a ProviderSettings view loaded with an "anthropic" provider configured via OAuth with backend masked_key None
    let infos = project_display_infos(&[oauth_credential("anthropic", "Anthropic")], &[]);

    // @step And the "anthropic" provider row is expanded
    // @step When the nav tree is rendered into a buffer
    let buf = render_view_expanded(infos, "anthropic");

    // @step Then the anthropic provider header row contains "Anthropic ✓ OAuth [Claude]"
    assert!(
        any_row_contains(&buf, "Anthropic ✓ OAuth [Claude]"),
        "expected synthetic OAuth header annotation"
    );

    // @step And no row in the buffer contains "Anthropic (not configured)"
    assert!(
        !any_row_contains(&buf, "Anthropic (not configured)"),
        "OAuth-logged-in provider must NEVER show (not configured)"
    );

    // @step And a separate child row contains "Logout from OAuth [Anthropic]"
    assert!(
        any_row_contains(&buf, "Logout from OAuth [Anthropic]"),
        "expected separate OAuth logout child row"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: OAuth-logged-in codex header matches the screenshot case
// ────────────────────────────────────────────────────────────────────────

#[test]
fn oauth_logged_in_codex_header_matches_screenshot_case() {
    // @step Given a ProviderSettings view loaded with a "codex" provider configured via OAuth with backend masked_key None
    let infos = project_display_infos(&[oauth_credential("codex", "Codex (ChatGPT)")], &[]);

    // @step When the nav tree is rendered into a buffer
    let buf = render_view(infos);

    // @step Then the codex provider header row contains "Codex (ChatGPT) ✓ OAuth [ChatGPT]"
    assert!(
        any_row_contains(&buf, "Codex (ChatGPT) ✓ OAuth [ChatGPT]"),
        "expected codex synthetic OAuth header annotation"
    );
}
