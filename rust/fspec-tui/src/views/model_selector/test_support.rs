//! Shared test fixtures for the model_selector test suite.
//!
//! Extracted (PROV-107) so the per-feature test files
//! (`tests_core`, `tests_scroll`, `tests_current_model`, `tests_collapse`,
//! `tests_tab`, `tests_crud_add`, `tests_crud_delete`) can each stay under
//! the 300-LoC ceiling while sharing one copy of the key/model/provider/
//! view builders. All helpers are `pub(crate)` so the sibling `#[path]`
//! test modules can `use super::test_support::*;`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use codelet_rpc_types::ModelEntry;
use crossterm::event::{KeyEventKind, KeyEventState};

pub(crate) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

pub(crate) fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 200_000,
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }
}

pub(crate) fn provider(key: &str, ids: &[&str]) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: key.to_string(),
        models: ids.iter().map(|i| model(i)).collect(),
        profile_name: None,
        is_unreachable: false,
    }
}

pub(crate) fn loaded_view() -> ModelSelectorView {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    // Most of these shipped scenarios assume their GIVEN precondition of
    // expanded provider groups (the pre-RPC-342 default). RPC-342 makes the
    // real default collapse-on-load — verified separately by its own tests
    // below — so here we restore the expanded fixture explicitly.
    expand_all(&mut v);
    v
}

/// Test fixture helper: expand every provider section and reset the
/// selection to the first selectable row. Mirrors the pre-RPC-342
/// all-expanded default for scenarios whose GIVEN assumes expanded groups.
pub(crate) fn expand_all(v: &mut ModelSelectorView) {
    v.expanded = v.providers.iter().map(|p| p.key.clone()).collect();
    v.rebuild_rows();
    v.anchor_first_selectable();
    v.adjust_scroll();
}

/// Render into a `width`x`height` TestBackend so `self.visible_rows`
/// is populated from the real body height (height - chrome - legend).
pub(crate) fn render_at(v: &mut ModelSelectorView, width: u16, height: u16) {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).expect("term");
    term.draw(|f| v.render(f.area(), f.buffer_mut()))
        .expect("draw");
}

pub(crate) fn tall_view() -> ModelSelectorView {
    // One provider, 30 models → a single header + 30 selectable rows.
    let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider("openai", &refs)]);
    expand_all(&mut v);
    v
}

pub(crate) fn custom_model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 128_000,
        supports_reasoning: false,
        supports_vision: false,
        is_custom: true,
    }
}

pub(crate) fn profile_provider_with(
    key: &str,
    profile: &str,
    models: Vec<ModelEntry>,
) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: format!("{key}: {profile}"),
        models,
        profile_name: Some(profile.to_string()),
        is_unreachable: false,
    }
}

/// A single collapsed profile section with one custom model; cursor rests
/// on the (non-selectable) profile header.
pub(crate) fn profile_header_view() -> ModelSelectorView {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![profile_provider_with(
        "openai",
        "my-profile",
        vec![custom_model("c1")],
    )]);
    v
}

/// An EXPANDED profile section with a built-in model then a custom model;
/// cursor rests on the first selectable (built-in) model row.
pub(crate) fn expanded_profile_view() -> ModelSelectorView {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![profile_provider_with(
        "openai",
        "my-profile",
        vec![model("base"), custom_model("mycustom")],
    )]);
    v.expanded = ["openai".to_string()].into_iter().collect();
    v.rebuild_rows();
    v.anchor_first_selectable();
    v.adjust_scroll();
    v
}
