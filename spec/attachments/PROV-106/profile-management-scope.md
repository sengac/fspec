# PROV-106 — Wire profile create / edit / per-profile delete (Rust frontend)

## Status
Follow-up card from the `provider-settings-parity` epic (recorded at completion of
PROV-101..104). **Card creation only — not yet specified/implemented.**

## Problem (verified in code, not fabricated)

The Rust Provider Settings nav tree renders profile rows and an `AddProfile`
("+ Add profile") row, and PROV-100 already *loads* custom OpenAI profiles from
`~/.fspec/fspec-config.json` (user+project merge). **But profile mutation has no
modes — only read + explicit no-ops.**

In `codelet/fspec-tui/src/views/provider_settings/list_actions.rs`:

```rust
// PROV-102 headline fix: open THIS profile's provider Detail/models view ...
NavItemKind::Profile { .. } => {
    view.mode = ProviderSettingsMode::Detail { provider_id, sub: DetailSub::Summary { last_status: None } };
    ...
}
// TS initializeNewProfile opens a profile-creation form; the Rust
// port has no such mode yet. Consume explicitly ...
NavItemKind::AddProfile => ProviderSettingsEvent::Consumed,
```

And in `delete_on_nav_item`:

```rust
NavItemKind::Profile { .. } | NavItemKind::AddProfile | NavItemKind::OAuthLogin { .. } => {
    ProviderSettingsEvent::Consumed   // no per-profile delete
}
```

Header comment (`list_actions.rs:17-22`) records the parity gap explicitly:
> the Rust frontend has no profile-create, profile-edit, OAuth-login,
> OAuth-disconnect or per-profile-delete modes.

So today:
- **Profile create** (Enter on `AddProfile` → TS `initializeNewProfile`): no-op.
- **Profile edit** (Enter on a `Profile` row): opens a read-only `Detail/Summary`,
  no edit form.
- **Per-profile delete** (`d` on a `Profile` row): no-op.

Backend persistence already exists (RPC-346 `save/delete_custom_model` in
`profile_sections.rs`, whole-file read-modify-write over a `preserve_order`
`serde_json::Value`, openai-guarded) and a CRUD form exists for the **model
selector** (RPC-344 `CustomModelForm` in `model_selector/form.rs`). This card
brings the equivalent CRUD UX into the **Provider Settings** profile rows.

## TS reference
- `src/tui/profile-management.ts` (`initializeNewProfile`, edit, delete)
- `src/tui/customModelCrudService.ts`
- `src/tui/provider-config.ts`
- `src/tui/inputHandlers/listModeHandler.ts:118-177`

## Scope
1. **Profile create form** (`AddProfile` Enter): form to enter profile fields
   (name/displayName, base URL, api style/facade, etc.), validate, persist to
   `~/.fspec/fspec-config.json` via existing backend, refresh `list_providers()`.
2. **Profile edit form** (`Profile` Enter, or a dedicated key): prefill from the
   stored profile, save changes, refresh.
3. **Per-profile delete** (`Profile` `d`): confirm dialog → delete from config →
   refresh.
4. New `ProviderSettingsMode` / `DetailSub` variant(s) for the profile form;
   reuse `CustomModelForm` patterns where sensible.

## Out of scope
- OAuth login/disconnect (→ PROV-105).
- Megafile refactor (→ PROV-107).

## Constraints / gates (ACDD)
- Strict 100% ACDD: feature file → tests → impl.
- Tests fully OFFLINE — temp `~/.fspec` via path-injectable config dir; full
  write→read→modify→write cycle; no env mutation.
- Files < 300 LoC (watch `provider_settings/mod.rs` at 296 — may need PROV-107
  extraction first or concurrent split); clippy `-D warnings`; cargo fmt clean.
- Parity verified against TS reference.
- **NO git** (user directive). Work directly in the working tree.

## Estimation note
Create + edit + delete is likely 8–13 points. Re-estimate after Example Mapping.
