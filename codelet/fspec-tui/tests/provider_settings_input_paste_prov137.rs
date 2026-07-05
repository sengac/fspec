//! PROV-137 — Paste support for /provider view input areas (profile form +
//! inline API-key entry), with the API key staying masked on paste.
//!
//! Feature: spec/features/provider-settings-input-paste.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment whose text is copied verbatim from
//! the feature file.
//!
//! Strategy: seed a `ProviderSettingsView` directly into the relevant input
//! mode (CreateProfile / Detail::EditApiKey / List), then drive the new
//! `ProviderSettingsView::handle_paste(text)` seam. For masking assertions the
//! whole view is rendered into a ratatui `Buffer` via the public `render`
//! entry (which routes through `body_render` → `profile_form_render` /
//! `detail::render_detail`) and the rendered rows are inspected for bullet
//! dots and the absence of the plaintext secret.
//!
//! RED PHASE: `ProviderSettingsView::handle_paste` does NOT exist yet, so this
//! file fails to compile ("no method named handle_paste"). That is the correct
//! red state. No production source file is touched by this test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn area() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    }
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
        masked_key: None,
        source: None,
    }
}

/// A create form past the name step, focused on `field_index`, with a real
/// name so the paste lands only in the targeted field.
fn form_on_field(field_index: usize) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    // Start the targeted field empty so the paste is the only content.
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

fn form_of(view: &ProviderSettingsView) -> &ProfileForm {
    match &view.mode {
        ProviderSettingsMode::CreateProfile { form, .. }
        | ProviderSettingsMode::EditProfile { form, .. } => form,
        other => panic!("expected a form mode, got {other:?}"),
    }
}

fn render_to_buffer(view: &mut ProviderSettingsView) -> Buffer {
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

/// Whole-buffer text (all rows joined with newlines).
fn buffer_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_string(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Count the total number of `•` bullet cells across the whole rendered buffer.
fn bullet_count(buf: &Buffer) -> usize {
    buffer_text(buf).matches('•').count()
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Pasting a URL into the focused Base URL field inserts it
// ════════════════════════════════════════════════════════════════════════
#[test]
fn pasting_a_url_into_the_focused_base_url_field_inserts_it() {
    // @step Given the profile create form is open with the Base URL field focused
    let mut view = create_view(form_on_field(0));

    // @step When I paste the text "https://api.example.com"
    view.handle_paste("https://api.example.com");

    // @step Then the Base URL field contains "https://api.example.com"
    assert_eq!(
        form_of(&view).field_value(0),
        "https://api.example.com",
        "pasted URL should populate the focused Base URL field"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Pasting an API key into the profile form keeps the field masked
// ════════════════════════════════════════════════════════════════════════
#[test]
fn pasting_an_api_key_into_the_profile_form_keeps_the_field_masked() {
    // @step Given the profile create form is open with the API Key field focused
    let mut view = create_view(form_on_field(1));

    // @step When I paste the text "sk-secret123"
    view.handle_paste("sk-secret123");

    // @step Then the API Key field stores the value "sk-secret123"
    assert_eq!(
        form_of(&view).field_value(1),
        "sk-secret123",
        "pasted secret should be stored verbatim in the API Key field"
    );

    // @step And the API Key field renders 12 bullet dots and not the plaintext secret
    let buf = render_to_buffer(&mut view);
    let text = buffer_text(&buf);
    assert!(
        !text.contains("sk-secret123"),
        "rendered form must NOT show the plaintext secret, got {text:?}"
    );
    assert_eq!(
        bullet_count(&buf),
        12,
        "12-char API key should render as 12 bullet dots, got {} in {text:?}",
        bullet_count(&buf)
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Pasting a multi-line secret into the inline API-key entry strips
//           newlines and stays masked
// ════════════════════════════════════════════════════════════════════════
#[test]
fn pasting_a_multi_line_secret_into_the_inline_api_key_entry_strips_newlines_and_stays_masked() {
    // @step Given the inline API-key entry is open with an empty draft
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "anthropic".to_string(),
        sub: DetailSub::EditApiKey {
            draft: String::new(),
        },
    };

    // @step When I paste the text "sk-abc\ndef"
    // (the feature step text carries a literal backslash-n; the pasted &str
    // here contains a REAL embedded newline that must be stripped.)
    view.handle_paste("sk-abc\ndef");

    // @step Then the API-key draft contains "sk-abcdef"
    match &view.mode {
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { draft },
            ..
        } => assert_eq!(
            draft, "sk-abcdef",
            "newline must be stripped from the pasted draft, got {draft:?}"
        ),
        other => panic!("expected Detail::EditApiKey, got {other:?}"),
    }

    // @step And the inline API-key entry renders 9 bullet dots and not the plaintext secret
    let buf = render_to_buffer(&mut view);
    let text = buffer_text(&buf);
    assert!(
        !text.contains("sk-abcdef") && !text.contains("sk-abc"),
        "rendered inline entry must NOT show the plaintext secret, got {text:?}"
    );
    assert_eq!(
        bullet_count(&buf),
        9,
        "9-char draft should render as 9 bullet dots, got {} in {text:?}",
        bullet_count(&buf)
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Pasting while the provider list is focused does nothing
// ════════════════════════════════════════════════════════════════════════
#[test]
fn pasting_while_the_provider_list_is_focused_does_nothing() {
    // @step Given the provider list is focused with no input field open
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.mode = ProviderSettingsMode::List;

    // @step When I paste the text "https://api.example.com"
    let ev = view.handle_paste("https://api.example.com");

    // @step Then the paste is ignored and no field value changes
    assert!(
        matches!(ev, ProviderSettingsEvent::Ignored),
        "paste on the List mode must return Ignored"
    );
    assert_eq!(
        view.mode,
        ProviderSettingsMode::List,
        "mode must stay List and no field value may change"
    );
}
