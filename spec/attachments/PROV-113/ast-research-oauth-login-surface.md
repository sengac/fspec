# PROV-113 AST Research — OAuth login wiring surface

Performed with the AstGrep tool against the existing PROV-112 OAuth seams to
locate the exact extension points for the login flows (browser/headless/device).

## ProviderSettingsMode enum (mode variants to extend)
- `pub enum ProviderSettingsMode { ... }` → fspec-tui/src/views/provider_settings/mod.rs:46
  Currently: List, Detail{provider_id,sub}, CreateProfile, EditProfile,
  DisconnectOAuth{provider_id}. PROV-113 adds OAuthBrowserWaiting,
  OAuthDeviceWaiting, OAuthHeadlessCodeEntry, OAuthSuccess, OAuthError.
  Decision: move enum + DetailSub into a sibling `mode.rs` to keep mod.rs <300 LoC.

## FspecBackend OAuth trait method seam (mirror for the 5 new login methods)
- trait default  : transport/mod.rs:609  oauth_clear_tokens(&self,_provider_id)->Result<()>
- embedded fwd   : transport/embedded.rs:660 forwards to self.client.oauth_clear_tokens(ctx,id)
  PROV-113 mirrors this exact default+forward pattern for oauth_browser_login,
  oauth_headless_start, oauth_headless_complete, oauth_device_start, oauth_device_poll
  plus supports_browser_oauth() (default false, embedded true).

## RPC FspecService delegation seam
- rpc/src/lib.rs:447 oauth_clear_tokens trait method; :1615 impl delegates to
  crate::oauth_disconnect::clear_oauth_tokens. PROV-113 adds sibling
  oauth_login.rs and delegating impls (codelet-providers DIRECT, not napi).

## Dispatch + Action seam
- dispatch_provider_settings_oauth.rs:27 try_dispatch_oauth (PROV-112) extended.
- components/mod.rs:739 OAuthDisconnect{provider_id} — sibling for new start/fold Actions.
- ProviderCredentialsLoaded fold (dispatch_provider_settings_profiles.rs:26) does NOT
  reset mode → safe to leave OAuthSuccess mode while post-login list refresh rebuilds nav.

## Nav row gating seam (scenario 7)
- nav_item.rs:79 build_nav_items is pure (providers,expanded,filter). To avoid
  changing its widely-used signature, browser gating is applied as a retain() in
  rebuild_nav_items (nav_tree_ops.rs:122) keyed on a new view flag.
