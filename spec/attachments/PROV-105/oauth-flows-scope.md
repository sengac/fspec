# PROV-105 — Wire real OAuth login & disconnect flows (Rust frontend)

## Status
Follow-up card from the `provider-settings-parity` epic (recorded at completion of
PROV-101..104). **Card creation only — not yet specified/implemented.**

## Problem (verified in code, not fabricated)

The Rust Provider Settings nav tree already *projects* OAuth rows correctly
(`projection.rs`): per-provider login methods (`oauth_login_methods`), an
`oauth_status_label` ("Logout from OAuth [Anthropic]"), and `[Claude]`/`[ChatGPT]`
source labels. **But the action dispatch is a placeholder.**

In `codelet/fspec-tui/src/views/provider_settings/list_actions.rs`:

```rust
// OAuth login / disconnect are not yet implemented in the Rust
// frontend; route to the honest OAuthNotice placeholder keyed by the
// row's OWN provider_id (TS: startLogin / disconnect-oauth).
NavItemKind::OAuthLogin { .. } | NavItemKind::OAuthStatus { .. } => {
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::OAuthNotice,
    };
    ...
}
```

- **OAuth login** (Enter on an `OAuthLogin` row → TS `startLogin`): browser flow
  and headless "Sign in with code" flow are NOT performed. Row opens
  `DetailSub::OAuthNotice` placeholder.
- **OAuth disconnect/logout** (Enter on `OAuthStatus` row → TS `disconnect-oauth`,
  or `d` which currently collapses to the delete-credentials confirm): the real
  token-disconnect is NOT performed.

The placeholders are honest — they carry the **correct** `provider_id` (this is
the PROV-102 fix), so wiring the real flow is purely additive.

## TS reference
- `src/tui/inputHandlers/listModeHandler.ts:118-177` (Enter/`d` dispatch)
- `src/tui/provider-config.ts`, `profile-management.ts`
- TS `startLogin` (browser + headless code), `disconnect-oauth`,
  `buildOauthLoginNavItems`, `oauthProviderLabels.ts`

## Scope
1. **OAuth login — browser**: trigger the provider's browser-based OAuth flow,
   handle the redirect/callback, persist tokens (anthropic → `claude_auth.json`;
   codex/github-copilot equivalents), refresh `list_providers()`.
2. **OAuth login — headless ("Sign in with code")**: device/code paste flow for
   anthropic + github-copilot, persist tokens, refresh.
3. **OAuth disconnect**: remove stored OAuth tokens for the provider, refresh the
   nav tree so the row flips back to login methods.
4. Replace `DetailSub::OAuthNotice` placeholder routing with real mode(s).
5. Backend RPC/NAPI surface as needed (mirror RPC-054 credential-write pattern).

## Out of scope
- Profile create/edit/delete (→ PROV-106).
- Megafile refactor (→ PROV-107).

## Constraints / gates (ACDD)
- Strict 100% ACDD: feature file → tests → impl.
- Tests fully OFFLINE — no real OAuth network; inject the token-exchange boundary
  via a path-injectable / faked transport. Use dummy credentials, no env mutation.
- Files < 300 LoC; clippy clean (`-D warnings`); cargo fmt clean; build incl.
  downstream core+napi.
- Parity verified against the TS reference above.
- **NO git** (user directive). Work directly in the working tree.

## Estimation note
Browser + headless + disconnect across 3 OAuth providers (anthropic, codex,
github-copilot) is likely > 13 points. Re-estimate after Example Mapping and
split per-flow if needed.
