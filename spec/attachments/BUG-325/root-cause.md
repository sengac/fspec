# BUG-325 root cause: custom-model writes under an OpenAI profile fail silently

## Problem

Adding, editing or deleting a custom model in the TUI model selector under an
OpenAI profile section does nothing, and the user is told nothing.

Evidence chain:

1. The daemon keys every profile section by `"{provider_id}:{profile_name}"`:
   `rust/sessions/src/profile_sections.rs:72-79` (`build_profile_provider_info`)
   called from `profile_sections.rs` `build_local_profile_sections` and
   `rust/sessions/src/inherited_profile_sections.rs:23`. The wire `ProviderInfo`
   (`rust/rpc-types/src/dialogs.rs:210-226`) has no field naming the base
   provider, so `key` is the only provider identity a client has.
2. The model selector copies that key into every row:
   `rust/fspec-tui/src/views/model_selector/rows.rs:83` (header) and `:101`
   (model rows), `provider_key: provider.key.clone()`.
3. The CRUD forms send the row key as the provider id:
   `rust/fspec-tui/src/views/model_selector/crud.rs:25` (Add), `:45` (Edit),
   `:74` (Delete) — `provider_id = row.provider_key`, i.e. `"openai:my-profile"`.
4. The daemon rejects it: `rust/sessions/src/handle_impl/handle_impl_6.rs:110`
   (add) and `:126` (update) return `Err("Profiles are only supported for the
   OpenAI API provider (got 'openai:my-profile')")`. Delete (`:140-148`) has no
   guard; `profile_object_mut` (`profile_sections.rs:376-392`) returns `None`
   for a non-"openai" id and `delete_custom_model_at` (`:438-449`) returns
   `Ok(())` without writing — a silent no-op.
5. The TUI discards every result: `rust/fspec-tui/src/app/dispatch_model_selector.rs:100-127`
   uses `let _ = backend.add_custom_model(..)` (and update/delete), then
   re-lists providers, so the list simply comes back unchanged.
6. Tests passed because the fixtures
   (`rust/fspec-tui/src/views/model_selector/test_support.rs:35-115`) build
   profile sections with key `"openai"`, a shape the daemon never sends.

## Design

- Add an explicit base-provider field to the wire `ProviderInfo`:
  `provider_id: String`, appended after `is_unreachable` (bincode position 6).
  The daemon fills it everywhere it builds a `ProviderInfo`: the base
  provider id for profile sections (`"openai"`), and the same id as `key` for
  cloud/custom provider entries. `key` keeps its meaning (unique section key,
  used by the selector's expand/collapse set and by `set_session_model`).
- The model selector rows carry `provider_id` from `ProviderInfo.provider_id`;
  Add/Edit/Delete send `row.provider_id`. Nothing parses `"openai:<profile>"`
  and nothing falls back to `provider_key`.
- The daemon's `delete_custom_model` rejects a non-openai provider id with the
  same error as add/update. Writes to a profile that does not exist (or a
  missing/malformed config file, or an edit whose original id is absent, or a
  delete whose id is absent) return an error instead of `Ok(())` — they are the
  same silent-no-op class as the original bug.
- The TUI reports a failed write: it logs it (`tracing::warn!`) and shows
  `[error] <op> custom model failed: <reason>` inside the model selector (the
  full-screen view, where the user is looking) and, via the existing
  `Action::EmitSessionNotice` mechanism, in the session's scrollback.
- Fixtures use the real daemon key shape (`"openai:<profile>"`, provider id
  `"openai"`).

## Alternatives rejected

- Parsing the base provider out of `key` (split on `:`) in the TUI: encodes a
  daemon formatting convention in every client (the Paseo plugin would need it
  too) and is a fallback that breaks if the key format changes.
- Making the daemon accept `"openai:<profile>"` as provider id: two spellings
  for one parameter; hides the client bug instead of fixing the contract.
- Changing `key` to `"openai"` for profile sections: keys would no longer be
  unique, breaking expand/collapse and model selection.

## Wire / API changes

- `codelet_rpc_types::ProviderInfo` gains `provider_id: String` as its last
  field. Full order: `key, display_name, models, profile_name, is_unreachable,
  provider_id`.
- `delete_custom_model` now returns `Err` for a non-openai provider id, a
  missing profile, or a model id that is not present (was `Ok(())`).
- `add_custom_model`/`update_custom_model` return `Err` for a missing profile
  or config (was `Ok(())`); `update_custom_model` returns `Err` when the
  original model id is not present.
