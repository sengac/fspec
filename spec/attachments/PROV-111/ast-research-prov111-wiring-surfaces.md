# PROV-111 — AST research: wiring surfaces

AST-grep over `codelet/fspec-tui/src/views/provider_settings/` (rust) to locate
the existing functions PROV-111 must extend/modify.

## Existing display-string loader (mirror target for the full-config loader)
- `profiles_config.rs:70` — `pub fn load_openai_profiles_from(user_config_dir: &Path, project_root: &Path) -> Vec<String>`
  PROV-111 adds a sibling `load_openai_profile_configs_from(user, project) -> BTreeMap<String, ProfileDefinition>`
  reusing the same `providers.openai.profiles` JSON walk + project-over-user merge.

## Enter / d dispatch arms to rewrite (list_actions.rs)
- `enter_on_nav_item` `NavItemKind::Profile { .. }` arm (currently opens Detail/Summary placeholder) -> EditProfile.
- `enter_on_nav_item` `NavItemKind::AddProfile` arm (currently `Consumed` no-op) -> CreateProfile.
- `delete_on_nav_item` `Profile | AddProfile | OAuthLogin` arm (currently `Consumed`) -> Profile opens per-profile confirm.

## View state (mod.rs)
- `ProviderSettingsView` struct — add `profile_configs` + `pending_profile_delete` fields.
- `handle_key` delete_confirm Primary arm — branch on `pending_profile_delete`.

## Dispatch fold (dispatch_provider_settings.rs)
- `handle_provider_credentials_loaded` — additionally load + store the per-profile config map.

## Test to update
- `tests/prov102_nav_item_action_dispatch.rs` — `enter_on_openai_profile_*` and
  `d_on_profile_row_leaves_confirm_closed` flip to EditProfile / per-profile confirm.
