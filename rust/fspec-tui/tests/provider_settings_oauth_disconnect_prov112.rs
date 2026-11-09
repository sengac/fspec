//! PROV-112 — OAuth disconnect/logout: confirm-flow view tests, App-level
//! dispatch tests, and the transport-surface test, all for one feature.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//!
//! One test file per feature (ACDD 1:1). Sections:
//!   1. VIEW/KEY — pure `ProviderSettingsView::handle_key` drives (no backend).
//!   2. DISPATCH — `App::dispatch` against the shared `MockBackend`
//!      (call counters + scripted Ok/Err), fully offline.
//!   3. TRANSPORT — embedded forwards to the providers-direct OAuth wiring;
//!      websocket forwards the same operations to the daemon.
//!
//! No real OAuth network and no real `~/.fspec` mutation: the transport test
//! redirects `FSPEC_HOME` to a throwaway tempdir, and this binary hosts the
//! only `FSPEC_HOME` writer so that env mutation cannot race the other tests.

use std::sync::Arc;

use anyhow::Result;
use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::{
    OAuthMethod, ProviderDisplayInfo, ProviderSettingsEvent, ProviderSettingsMode,
    ProviderSettingsView,
};
use codelet_fspec_tui::{App, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc_types::{ProviderCredentialInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

mod common;
use common::MockBackend;

// ─────────────────────────── shared key helpers ───────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn char_key(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

#[path = "prov112/view_keys.rs"]
mod view_keys;

#[path = "prov112/dispatch.rs"]
mod dispatch;

#[path = "prov112/transport.rs"]
mod transport;
