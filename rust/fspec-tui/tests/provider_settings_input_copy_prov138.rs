//! PROV-138 — Copy support for /provider view input areas (Ctrl+C copies the
//! focused field via OSC 52), with the API key copied MASKED.
//!
//! Feature: spec/features/provider-settings-input-copy.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching verbatim `// @step` comment.
//!
//! Test levels (see report):
//!   * VIEW-LEVEL (security-critical): drive the view's Ctrl+C entrypoint
//!     directly and assert it returns
//!     `ProviderSettingsEvent::Emit(Action::CopyToClipboard(expected))` where
//!     `expected` is the MASKED (API key) or PLAINTEXT (other field) string —
//!     so the plaintext secret is proven to never enter the action bus.
//!   * APP-LEVEL (end-to-end OSC 52 bytes): for the two masked scenarios,
//!     inject a `Vec<u8>` clipboard writer via
//!     `App::set_clipboard_writer_for_test`, drive the provider-view Ctrl+C
//!     through the real `App::handle_event` path, and assert the emitted
//!     OSC 52 payload base64-decodes to the bullet dots and NEVER contains the
//!     plaintext secret substring.
//!
//! RED PHASE: `ProviderSettingsView` has no Ctrl+C copy branch yet — the
//! blanket CONTROL/ALT arm (mod.rs:190-195) consumes Ctrl+C and returns
//! `Consumed` (no `Emit(CopyToClipboard)`). Every assertion that a
//! `CopyToClipboard` is emitted / lands on the clipboard therefore FAILS.
//! No production source file is touched by this test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

mod common;
use common::MockBackend;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

/// A Ctrl+C key-press event (no other modifiers).
fn ctrl_c() -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 1,
        masked_key: None,
        source: None,
    }
}

/// A create form past the name step, focused on `field_index`.
fn form_on_field(field_index: usize) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = String::new();
    form.api_key = String::new();
    form.field_index = field_index;
    form
}

fn create_view(form: ProfileForm) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.mode = ProviderSettingsMode::CreateProfile {
        provider_id: "openai".to_string(),
        form,
    };
    view
}

/// The CopyToClipboard payload of an `Emit`, or None for any other event.
fn copied_text(ev: &ProviderSettingsEvent) -> Option<String> {
    match ev {
        ProviderSettingsEvent::Emit(Action::CopyToClipboard(text)) => Some(text.clone()),
        _ => None,
    }
}

// --- App-level clipboard harness (mirrors COPY-006 tests) -----------------

/// An `Arc<Mutex<Vec<u8>>>`-backed writer so the test can inspect the exact
/// clipboard bytes after driving a copy through the App.
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("clipboard buffer mutex")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Build an App flipped into the provider-settings view with an injected
/// clipboard sink. Returns the App and the shared clipboard buffer.
fn app_in_provider_view() -> (App, Arc<Mutex<Vec<u8>>>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    // Flip active_view → ProviderSettings so App::handle_event routes Ctrl+C
    // to the provider view (no tokio runtime needed for the view seam).
    app.dispatch(Action::OpenProviderSettingsView);
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    app.set_clipboard_writer_for_test(Box::new(SharedWriter(buf.clone())));
    (app, buf)
}

/// Drain + dispatch every queued Action so an emitted CopyToClipboard runs its
/// reducer synchronously.
fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

fn clip_bytes(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    buf.lock().expect("clipboard buffer mutex").clone()
}

/// The OSC 52 sequence the App emits for `text`: `ESC ] 52 ; c ; <base64> BEL`.
fn osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Copying the focused Base URL field copies its plaintext value
// ════════════════════════════════════════════════════════════════════════
#[test]
fn copying_the_focused_base_url_field_copies_its_plaintext_value() {
    // @step Given the profile create form is open with the Base URL field focused and containing "https://api.example.com"
    let mut form = form_on_field(0);
    form.base_url = "https://api.example.com".to_string();
    let mut view = create_view(form);

    // @step When I press Ctrl+C
    let ev = view.handle_key(ctrl_c());

    // @step Then the clipboard receives "https://api.example.com"
    assert_eq!(
        copied_text(&ev).as_deref(),
        Some("https://api.example.com"),
        "Base URL Ctrl+C must emit CopyToClipboard with the plaintext URL, got {ev:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Copying the profile form API Key field copies the masked value
// ════════════════════════════════════════════════════════════════════════
#[test]
fn copying_the_profile_form_api_key_field_copies_the_masked_value() {
    // @step Given the profile create form is open with the API Key field focused and containing "sk-secret123"
    let mut form = form_on_field(1);
    form.api_key = "sk-secret123".to_string();
    let mut view = create_view(form);

    // @step When I press Ctrl+C
    let ev = view.handle_key(ctrl_c());

    // @step Then the clipboard receives 12 bullet dots and not the plaintext secret
    let text =
        copied_text(&ev).unwrap_or_else(|| panic!("expected CopyToClipboard emit, got {ev:?}"));
    assert_eq!(text, "•".repeat(12), "API key must copy 12 bullet dots");
    assert!(
        !text.contains("sk-secret123"),
        "the plaintext secret must NEVER reach the clipboard action, got {text:?}"
    );

    // App-level end-to-end: the OSC 52 bytes must decode to the dots, never the secret.
    let (mut app, clip) = app_in_provider_view();
    {
        let mut f = form_on_field(1);
        f.api_key = "sk-secret123".to_string();
        app.navigator_mut().provider_settings.mode = ProviderSettingsMode::CreateProfile {
            provider_id: "openai".to_string(),
            form: f,
        };
    }
    let _ = app.handle_event(&Event::Key(ctrl_c()));
    drain(&mut app);
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52(&"•".repeat(12)),
        "clipboard OSC 52 bytes must carry the 12 masked dots"
    );
    assert!(
        !String::from_utf8_lossy(&bytes).contains("sk-secret123"),
        "the plaintext secret must NEVER appear in the OSC 52 clipboard bytes"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Copying the inline API-key entry copies the masked draft
// ════════════════════════════════════════════════════════════════════════
#[test]
fn copying_the_inline_api_key_entry_copies_the_masked_draft() {
    // @step Given the inline API-key entry is open with the draft "sk-abcdef"
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("anthropic")]);
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "anthropic".to_string(),
        sub: DetailSub::EditApiKey {
            draft: "sk-abcdef".to_string(),
        },
    };

    // @step When I press Ctrl+C
    let ev = view.handle_key(ctrl_c());

    // @step Then the clipboard receives 9 bullet dots and not the plaintext secret
    let text =
        copied_text(&ev).unwrap_or_else(|| panic!("expected CopyToClipboard emit, got {ev:?}"));
    assert_eq!(text, "•".repeat(9), "inline draft must copy 9 bullet dots");
    assert!(
        !text.contains("sk-abcdef"),
        "the plaintext draft must NEVER reach the clipboard action, got {text:?}"
    );

    // App-level end-to-end OSC 52 byte assertion.
    let (mut app, clip) = app_in_provider_view();
    app.navigator_mut()
        .provider_settings
        .set_providers(vec![pinfo("anthropic")]);
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::Detail {
        provider_id: "anthropic".to_string(),
        sub: DetailSub::EditApiKey {
            draft: "sk-abcdef".to_string(),
        },
    };
    let _ = app.handle_event(&Event::Key(ctrl_c()));
    drain(&mut app);
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52(&"•".repeat(9)),
        "clipboard OSC 52 bytes must carry the 9 masked dots"
    );
    assert!(
        !String::from_utf8_lossy(&bytes).contains("sk-abcdef"),
        "the plaintext draft must NEVER appear in the OSC 52 clipboard bytes"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Pressing Ctrl+C on the provider list copies no field value
// ════════════════════════════════════════════════════════════════════════
#[test]
fn pressing_ctrl_c_on_the_provider_list_copies_no_field_value() {
    // @step Given the provider list is focused with no input field open
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("anthropic")]);
    view.mode = ProviderSettingsMode::List;

    // @step When I press Ctrl+C
    let ev = view.handle_key(ctrl_c());

    // @step Then no field value is copied to the clipboard
    assert!(
        copied_text(&ev).is_none(),
        "Ctrl+C on the List must NOT emit CopyToClipboard, got {ev:?}"
    );

    // App-level: nothing must land on the clipboard sink.
    let (mut app, clip) = app_in_provider_view();
    app.navigator_mut()
        .provider_settings
        .set_providers(vec![pinfo("anthropic")]);
    app.navigator_mut().provider_settings.mode = ProviderSettingsMode::List;
    let _ = app.handle_event(&Event::Key(ctrl_c()));
    drain(&mut app);
    assert!(
        clip_bytes(&clip).is_empty(),
        "Ctrl+C on the List must write nothing to the clipboard"
    );
}
