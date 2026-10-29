use codelet_fspec_tui::views::agent::confirm_dialog::ConfirmDialog;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::provider_settings::{
    profile_form::ProfileForm, DetailSub, ProviderSettingsMode,
};
use codelet_fspec_tui::{App, ViewMode};
use crossterm::event::KeyCode;

use super::fixture::*;

pub async fn cancel_local_modes(app: &mut App) {
    key(app, KeyCode::Char('/')).await;
    key(app, KeyCode::Char('a')).await;
    key(app, KeyCode::Esc).await;
    assert_eq!(app.active_view(), ViewMode::ProviderSettings);
    assert!(app.navigator().provider_settings.filter.is_empty());
    assert!(!app.navigator().provider_settings.filter_mode);
    for mode in [
        ProviderSettingsMode::Detail {
            provider_id: "openai".into(),
            sub: DetailSub::EditApiKey {
                draft: "synthetic-key".into(),
            },
        },
        ProviderSettingsMode::CreateProfile {
            provider_id: "openai".into(),
            form: ProfileForm::new_create(),
        },
        ProviderSettingsMode::DisconnectOAuth {
            provider_id: "openai".into(),
        },
    ] {
        app.navigator_mut().provider_settings.mode = mode;
        key(app, KeyCode::Esc).await;
        assert_eq!(app.active_view(), ViewMode::ProviderSettings);
        if !matches!(
            app.navigator().provider_settings.mode,
            ProviderSettingsMode::List
        ) {
            key(app, KeyCode::Esc).await;
            assert_eq!(app.active_view(), ViewMode::ProviderSettings);
        }
        assert_eq!(
            app.navigator().provider_settings.mode,
            ProviderSettingsMode::List
        );
    }
    app.navigator_mut().provider_settings.delete_confirm = Some(ConfirmDialog::new(
        "Delete?",
        "Synthetic",
        "Delete",
        None,
        "Cancel",
    ));
    key(app, KeyCode::Esc).await;
    assert_eq!(app.active_view(), ViewMode::ProviderSettings);
    assert!(app.navigator().provider_settings.delete_confirm.is_none());
    key(app, KeyCode::Tab).await;
    key(app, KeyCode::Char('/')).await;
    key(app, KeyCode::Char('a')).await;
    key(app, KeyCode::Esc).await;
    assert_eq!(app.active_view(), ViewMode::ModelSelector);
}

pub async fn close_mux_panes(app: &mut App) -> TestResult {
    for kind in [MuxPaneKind::ChangedFiles, MuxPaneKind::Checkpoints] {
        let focus = app
            .navigator()
            .mux
            .effective_panes()
            .iter()
            .position(|pane| *pane == kind)
            .ok_or("tool pane missing")?;
        app.navigator_mut().mux.set_focus(focus);
        key(app, KeyCode::Esc).await;
        assert_eq!(app.active_view(), ViewMode::Mux);
        assert!(!app.navigator().mux.effective_panes().contains(&kind));
    }
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board, MuxPaneKind::Agent]
    );
    Ok(())
}
