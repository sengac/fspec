//! RPC-054 / PROV-113 — body-content render dispatch for
//! `ProviderSettingsView`.
//!
//! Extracted out of `mod.rs` so that module stays under the 300-LoC
//! budget. Mirrors the mode `match` arms of `handle_key`: every
//! `ProviderSettingsMode` variant renders its body here, delegating to
//! the per-mode renderers (`list`, `detail`, `profile_form_render`,
//! `oauth_confirm`, `oauth_login_render`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{
    detail, list, oauth_confirm, oauth_login_render, profile_form_render, ProviderSettingsMode,
    ProviderSettingsView,
};

/// Render the body region for the view's current mode. Captures the
/// body height into `visible_rows` (the body closure is the only place
/// the real available height is known).
pub(crate) fn render_mode_body(view: &mut ProviderSettingsView, body_area: Rect, buf: &mut Buffer) {
    view.visible_rows = body_area.height as usize;
    match &view.mode {
        ProviderSettingsMode::List => list::render_list(view, body_area, buf),
        ProviderSettingsMode::Detail { provider_id, sub } => {
            detail::render_detail(view, body_area, buf, provider_id, sub);
        }
        ProviderSettingsMode::CreateProfile { form, .. } => {
            profile_form_render::render_form(body_area, buf, "Create Profile", form);
        }
        ProviderSettingsMode::EditProfile { form, .. } => {
            profile_form_render::render_form(body_area, buf, "Edit Profile", form);
        }
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            oauth_confirm::render_disconnect_oauth(body_area, buf, provider_id);
        }
        ProviderSettingsMode::OAuthBrowserWaiting { provider_id } => {
            oauth_login_render::render_browser_waiting(body_area, buf, provider_id);
        }
        ProviderSettingsMode::OAuthDeviceWaiting {
            provider_id,
            user_code,
            verification_url,
        } => {
            oauth_login_render::render_device_waiting(
                body_area,
                buf,
                provider_id,
                user_code,
                verification_url,
            );
        }
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            authorize_url,
            code_input,
            ..
        } => {
            oauth_login_render::render_headless_code_entry(
                body_area,
                buf,
                authorize_url,
                code_input,
            );
        }
        ProviderSettingsMode::OAuthSuccess { provider_id } => {
            oauth_login_render::render_success(body_area, buf, provider_id);
        }
        ProviderSettingsMode::OAuthError { error, .. } => {
            oauth_login_render::render_error(body_area, buf, error);
        }
        // PROV-114: the github-copilot device preamble modes.
        ProviderSettingsMode::OAuthDeploymentTypeSelect { selected_index, .. } => {
            oauth_login_render::render_deployment_type_select(body_area, buf, *selected_index);
        }
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            url_input,
            validation_error,
            ..
        } => {
            oauth_login_render::render_enterprise_url_entry(
                body_area,
                buf,
                url_input,
                validation_error.as_deref(),
            );
        }
    }
}
