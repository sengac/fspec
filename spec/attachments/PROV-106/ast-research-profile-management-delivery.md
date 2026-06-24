# PROV-106 — AST research: delivered profile create/edit/delete surface (umbrella)

**Date:** 2026-06-23
**Purpose:** Records the actual AST-level OpenAI profile-management surface delivered
across the child slices. This umbrella split into PROV-108/109/110/111 (initial CRUD)
and was hardened by PROV-115 (compaction-threshold range parity) + PROV-116 (delete
cursor-return parity). All six children are DONE. Surface verified by grep/AST against
the working tree. See also the verified DeepSearch dossiers: profile-management-scope.md
and profile-edit-verified-findings.md.

## 1. Frontend form + modes
- `fspec-tui/src/views/provider_settings/profile_form.rs`:
  - `pub struct ProfileForm` :44 (name, base_url, api_key, context_window,
    max_output_tokens, compaction_threshold, field_index, is_editing_name, is_new).
  - `build_definition() -> Option<ProfileDefinition>` :153 (TS handleSave guard:
    base_url/api_key/trimmed-name required).
  - `profile_compaction_trigger()` :257 — PROV-115 profile-scoped range guard
    (1..=100 % / >=1000 tokens), leaving the shared model_selector splitter untouched.
- `fspec-tui/src/views/provider_settings/mode.rs`: `CreateProfile` :23, `EditProfile`
  :28 (per-profile delete uses the shared `delete_confirm` overlay + `pending_profile_delete`,
  not a dedicated mode).

## 2. Frontend routing + dispatch
- `list_actions.rs`: Enter on Profile → EditProfile (prefilled via profile_config_for);
  Enter on AddProfile → CreateProfile; `d` on Profile → per-profile delete confirm.
- `app/dispatch_provider_settings_profiles.rs`:
  - `handle_save_profile` :93 → backend.save_profile → list refresh.
  - `handle_delete_profile` :129 → backend.delete_profile; on Ok emits
    `Action::ProfileDeleteNavigate` :147 → `set_navigate_target` (PROV-116) → reload.
  - `handle_provider_credentials_loaded` reloads display slice + full per-profile
    ProfileConfig map + `apply_pending_navigate`.

## 3. Backend write path (path-injectable, openai-guarded, read-modify-write)
- `sessions/src/profile_persistence.rs`: `save_profile` :64 / `delete_profile` :82;
  cores `save_profile_at` :143 / `delete_profile_at` :178; `merge_profile` :118
  PRESERVES customModels + sibling keys (deliberate improvement over TS whole-object
  replace, PROV-106 rule [8]).
- `fspec-tui/.../profiles_config.rs`: `load_openai_profile_configs` (full per-profile
  ProfileDefinition map for edit prefill).

## 4. Delivery status
| Slice | Scope | Status |
|---|---|---|
| PROV-108 | backend save/delete profile write path | DONE |
| PROV-109 | transport + app dispatch wiring | DONE |
| PROV-110 | create/edit form UI | DONE |
| PROV-111 | nav routing, prefill, delete-confirm, end-to-end refresh | DONE |
| PROV-115 | compaction-threshold range-validation parity | DONE |
| PROV-116 | delete restores cursor to parent provider row (PROV-036 parity) | DONE |

All umbrella acceptance criteria are delivered by the children; this umbrella carries
no feature file of its own. Closure reflects child completion + the two parity-hardening
follow-ups discovered via fresh DeepSearch of the TS reference.
