# BUG-334 AST research

## fspec astgrep --pattern 'Action::ProviderSettingsStatus($$$A)' --lang rust --path rust/fspec-tui/src/app
rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:125:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:134:44:Action::ProviderSettingsStatus(format!("✗ {e}"))
rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:156:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:171:44:Action::ProviderSettingsStatus(format!("✗ {e}"))
rust/fspec-tui/src/app/dispatch_provider_settings_defaults.rs:21:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings_defaults.rs:45:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings_defaults.rs:53:52:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings_defaults.rs:61:44:Action::ProviderSettingsStatus(format!("✗ {error}"))
rust/fspec-tui/src/app/oauth_disconnect.rs:15:36:Action::ProviderSettingsStatus(
rust/fspec-tui/src/app/dispatch_provider_settings.rs:57:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings.rs:87:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings.rs:98:44:Action::ProviderSettingsStatus(format!("✗ {e}"))
rust/fspec-tui/src/app/dispatch_provider_settings.rs:123:44:Action::ProviderSettingsStatus(format!("✗ {e}"))
rust/fspec-tui/src/app/dispatch_provider_settings.rs:170:44:Action::ProviderSettingsStatus(format!("✗ {e}"))
rust/fspec-tui/src/app/dispatch_provider_settings.rs:203:44:Action::ProviderSettingsStatus(format!(
rust/fspec-tui/src/app/dispatch_provider_settings.rs:212:44:Action::ProviderSettingsStatus(format!("✗ {e}"))


## fspec astgrep --pattern 'view.status.clone()' --lang rust --path rust/fspec-tui/src/views/provider_settings
rust/fspec-tui/src/views/provider_settings/detail.rs:259:21:view.status.clone()


## fspec astgrep --pattern '&view.status' --lang rust --path rust/fspec-tui/src/views/provider_settings
rust/fspec-tui/src/views/provider_settings/body_render.rs:29:86:&view.status
rust/fspec-tui/src/views/provider_settings/body_render.rs:32:84:&view.status
rust/fspec-tui/src/views/provider_settings/body_render.rs:40:17:&view.status


## fspec astgrep --pattern 'list::render_list($$$A)' --lang rust --path rust/fspec-tui/src/views/provider_settings
rust/fspec-tui/src/views/provider_settings/body_render.rs:24:39:list::render_list(view, body_area, buf)


## Findings

- `view.status` is painted only by the form renderers (`body_render.rs:29,32,40`) and the
  Detail::Summary key entry (`detail.rs:259`). The List arm (`body_render.rs:24`) calls
  `list::render_list`, which never reads `view.status`.
- The OAuth logout failure (`oauth_disconnect.rs:15`) and several List-mode results (credential
  deletion and default-model errors in `dispatch_provider_settings.rs` /
  `dispatch_provider_settings_defaults.rs`) arrive while the view is in List mode, so none of them
  are visible.
