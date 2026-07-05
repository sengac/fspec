# PROV-136 — Cannot edit the name of an existing custom OpenAI profile

## Summary

In the Rust `/provider` profile edit form, the profile **name is locked** (non-editable) once a profile exists. You can only set the name at creation. The user wants to **rename an existing profile**. This is a **DELIBERATE divergence from the TS reference** (which also locks the name) — per explicit user request, we do NOT want name-edit parity with TS.

## Current (Locked) Behavior

`codelet/fspec-tui/src/views/provider_settings/profile_form.rs`. Two gate flags:

- **Create** (`new_create()`, lines 61–73): `is_editing_name: true`, `is_new: true` → name typeable.
- **Edit** (`from_definition()`, lines 77–92): `is_editing_name: false`, `is_new: false` → name locked.

The lock is enforced three ways:
1. `push_char` / `backspace` (lines 124–138) only mutate `self.name` when `is_editing_name` is true.
2. `move_up` (lines 114–122) can only set `is_editing_name = true` `if self.is_new` — false in edit mode, so the cursor can never return to the name row.
3. On save (line 210), edit mode passes `profile_name: Some(existing_name)`, so `form.name` is ignored entirely.

`ProfileDefinition` has **no name field** — the name is purely the map key under `providers.openai.profiles.<name>`.

## Backend Save Path

`Action::SaveProfile { provider_id, profile_name, definition }` is handled in
`codelet/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` — `handle_save_profile` → `backend.save_profile(provider_id, profile_name, definition)`.

Because the name is the map key, a naive "save under new name" would create a DUPLICATE (old key remains). Rename therefore requires **delete-old-key + write-new-key** semantics.

## Required Changes

### 1. Form: allow name editing in edit mode
- Allow the cursor to re-enter the name field in edit mode. Change `move_up`'s gate from `if self.is_new` to also permit edit mode (e.g. always allow re-entering name, or add an explicit `allow_name_edit` flag set true for both create and edit).
- Ensure `push_char` / `backspace` route to `self.name` while `is_editing_name` is true (already do — just needs `is_editing_name` reachable in edit mode).
- Keep the create-mode "name required" guard (`build_definition` already rejects empty trimmed name).

### 2. Track the original name for rename detection
- The edit form must remember the ORIGINAL profile name (the current map key) separately from the (possibly edited) `form.name`.
- On submit in edit mode, compare original vs new name:
  - **Unchanged** → normal save (overwrite same key).
  - **Changed** → rename: write under the new key AND delete the old key.

### 3. Save/dispatch: rename support
- Extend the save flow so a renamed edit performs delete-old + write-new atomically (read-modify-write of the config map).
- Options:
  - Add an `old_profile_name: Option<String>` to the save action/emit so `handle_save_profile` can delete the old key when it differs, OR
  - Introduce an explicit `Action::RenameProfile { provider_id, old_name, new_name, definition }`.
- Guard against name collisions: renaming onto an EXISTING profile name should be rejected (or explicitly confirmed) — decide during specifying.
- Preserve sibling keys not owned by `ProfileDefinition` (e.g. `customModels`) — the backend read-modify-write must not drop them during rename.

## Acceptance Criteria (Example-Mapping seeds)

- **Rule**: In edit mode, the user can move the cursor into the name field and edit it.
- **Rule**: Saving an edited profile with an unchanged name overwrites the same profile (no duplicate).
- **Rule**: Saving an edited profile with a changed name writes the new name and removes the old name (rename, not copy).
- **Rule**: A rename must not create a duplicate profile.
- **Rule**: A rename must preserve the profile's connection fields (baseUrl, apiKey, contextWindow, maxOutputTokens, compaction threshold) and any sibling keys like customModels.
- **Rule**: An empty/whitespace name cannot be saved (existing guard).
- **Rule (resolve during specifying)**: Renaming onto an existing profile name is rejected (no silent overwrite of a different profile).
- **Example**: Edit profile "work-vllm", change name to "work-vllm-2", save → config has "work-vllm-2" with the same fields and no "work-vllm".
- **Example**: Edit profile "work-vllm", leave name unchanged, change apiKey, save → still one profile "work-vllm" with updated apiKey.
- **Example**: Edit profile "work-vllm", clear the name, save → save is rejected (name required).

## Files In Scope

- `codelet/fspec-tui/src/views/provider_settings/profile_form.rs` (name-edit gate, original-name tracking)
- `codelet/fspec-tui/src/views/provider_settings/mode.rs` (EditProfile may need original_name; comment currently says "name fixed")
- `codelet/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` (rename save path)
- Possibly `codelet/fspec-tui/src/components/*` for the Action variant, and the provider backend save/rename API.
- Tests: form name-edit navigation, rename-writes-new-deletes-old, unchanged-name overwrite, sibling-key preservation, name-required guard.

## Explicit Divergence Note

This intentionally breaks parity with the TS implementation, which locks the name in edit mode. The user has explicitly requested name editing regardless of TS behavior. Update the "name fixed"/"name-editing gate" comments in `profile_form.rs` and `mode.rs` to reflect the new behavior.
