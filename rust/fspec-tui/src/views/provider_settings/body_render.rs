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
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Widget};

use crate::views::agent::text_wrap::wrapped_lines;

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
        ProviderSettingsMode::List => {
            let list_area = paint_list_status(view, body_area, buf);
            view.fit_window(list_area.height);
            list::render_list(view, list_area, buf);
        }
        ProviderSettingsMode::Detail { provider_id, sub } => {
            detail::render_detail(view, body_area, buf, provider_id, sub);
        }
        ProviderSettingsMode::CreateProfile { form, .. } => {
            profile_form_render::render_form(body_area, buf, "Create Profile", form, &view.status);
        }
        ProviderSettingsMode::EditProfile { form, .. } => {
            profile_form_render::render_form(body_area, buf, "Edit Profile", form, &view.status);
        }
        ProviderSettingsMode::EditSettings { provider_id, form } => {
            profile_form_render::render_form(
                body_area,
                buf,
                &format!("{provider_id} Settings"),
                form,
                &view.status,
            );
        }
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            oauth_confirm::render_disconnect_oauth(body_area, buf, provider_id);
        }
        ProviderSettingsMode::OAuthBrowserWaiting { provider_id } => {
            oauth_login_render::render_browser_waiting(
                body_area,
                buf,
                provider_id,
                view.oauth_browser_url.as_deref(),
                &view.status,
            );
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
                (user_code, verification_url),
                &view.status,
            );
        }
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            authorize_url,
            code_input,
            code_focused,
            ..
        } => {
            oauth_login_render::render_headless_code_entry(
                body_area,
                buf,
                authorize_url,
                (code_input, *code_focused),
                &view.status,
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

/// BUG-334: List mode paints `view.status` — the result of an operation
/// started from the list, e.g. a failed OAuth logout — at the body's bottom
/// and returns the rows left for the list. Without it a status set in
/// List mode was never visible. TUI-126: a long status wraps over at most
/// [`LIST_STATUS_ROWS`] rows instead of being cut off at the right edge.
fn paint_list_status(view: &ProviderSettingsView, area: Rect, buf: &mut Buffer) -> Rect {
    if view.status.is_empty() || area.height == 0 {
        return area;
    }
    let style = super::status_text::status_style(&view.status, Style::default());
    let lines = wrapped_lines(&view.status, usize::from(area.width), style);
    let height = (lines.len() as u16)
        .clamp(1, LIST_STATUS_ROWS)
        .min(area.height);
    let rows = Rect {
        y: area.y + area.height - height,
        height,
        ..area
    };
    Paragraph::new(lines).render(rows, buf);
    Rect {
        height: area.height - height,
        ..area
    }
}

/// TUI-126: the most rows a List-mode status takes.
const LIST_STATUS_ROWS: u16 = 3;
